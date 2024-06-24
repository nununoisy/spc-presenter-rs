use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use crate::apu::Apu;
use crate::script700::context::ImportContext;
use super::{lexer, parser::{self, script_area::{ScriptAst, Command, Condition, Operation, Parameter, ParameterValue}}, context::ScriptContext};

pub struct Runtime {
    emulator: *mut Apu,

    script_ast: ScriptAst,
    script_pc: usize,
    wait_cycles: u32,
    wait_port_event: Option<(bool, u8)>,
    wait_port_cycles: u32,

    cmp1: u32,
    cmp2: u32,
    working_memory: [u32; 8],

    call_stack: Vec<usize>,
    call_stack_enabled: bool,

    input_ports_buffer: [u8; 4],
    input_ports_unbuffered: bool,

    data_area: Vec<u8>,

    labels: HashMap<u16, (bool, usize)>
}

impl Runtime {
    pub fn new(emulator: *mut Apu) -> Box<Self> {
        Box::new(Self {
            emulator,

            script_ast: ScriptAst::new(),
            script_pc: 0,
            wait_cycles: 0,
            wait_port_event: None,
            wait_port_cycles: 0,

            cmp1: 0,
            cmp2: 0,
            working_memory: [0u32; 8],

            call_stack: vec![],
            call_stack_enabled: true,

            input_ports_buffer: [0u8; 4],
            input_ports_unbuffered: true,

            data_area: vec![],

            labels: HashMap::new()
        })
    }

    pub(super) fn emulator(&self) -> &mut Apu {
        unsafe {
            &mut (*self.emulator)
        }
    }

    pub fn reset(&mut self) {
        self.script_ast = ScriptAst::new();
        self.script_pc = 0;
        self.wait_cycles = 64;
        self.cmp1 = 0;
        self.cmp2 = 0;
        self.working_memory.fill(0);
        self.call_stack.clear();
        self.call_stack_enabled = true;
        self.input_ports_buffer.fill(0);
        self.input_ports_unbuffered = true;
    }

    fn read_script(&mut self, script: &str, import_context: ImportContext, file: &str) {
        let tokenize_result = lexer::tokenize(script, file);
        self.script_ast = parser::script_area::parse(&tokenize_result, import_context.clone());
        let parsed_data_area = parser::data_area::parse(&tokenize_result, import_context.clone());

        self.labels.clear();

        for (ptr, (command, context)) in self.script_ast.iter().enumerate() {
            match command {
                Command::Label { label } => {
                    if let Some((old_is_data_label, old_ptr)) = self.labels.insert(*label, (false, ptr)) {
                        let old_label_type_str: &'static str = if old_is_data_label { "data" } else { "script" };
                        println!("[Script700] {}: warning: script label :{} will replace existing {} label -> {}", context, label, old_label_type_str, old_ptr);
                    }
                },
                _ => ()
            }
        }
        for (label, context, ptr) in parsed_data_area.labels.iter() {
            if let Some((old_is_data_label, old_ptr)) = self.labels.insert(*label, (true, *ptr)) {
                let old_label_type_str: &'static str = if old_is_data_label { "data" } else { "script" };
                println!("[Script700] {}: warning: data label :{} will replace existing {} label -> {}", context, label, old_label_type_str, old_ptr);
            }
        }

        self.data_area = parsed_data_area.data;
        println!("[Script700] debug: parsing complete. script instructions: {}, data size: {}", self.script_ast.len(), self.data_area.len());
    }

    pub fn read_anonymous_script(&mut self, script: &str) {
        self.reset();
        self.read_script(script, ImportContext::AnonymousRoot, "<anonymous>")
    }

    pub fn load_script<P: AsRef<Path>>(&mut self, script_path: P) -> io::Result<()> {
        self.reset();

        let import_base_path = script_path.as_ref()
            .parent()
            .ok_or(io::Error::new(io::ErrorKind::NotFound, "Could not get base path!"))?
            .to_path_buf();

        let script = fs::read_to_string(&script_path)?;
        self.read_script(&script, ImportContext::Root(import_base_path), script_path.as_ref().file_name().unwrap().to_str().unwrap());

        Ok(())
    }

    fn read_parameter(&mut self, parameter: Parameter, context: Arc<ScriptContext>) -> u32 {
        match parameter {
            Parameter::Number(number) => number.get(self.cmp1, self.cmp2),
            Parameter::InputPort(port) => {
                let port = port.get(self.cmp1, self.cmp2) as usize;
                self.input_ports_buffer[port] as u32
            },
            Parameter::OutputPort(port) => {
                let port = port.get(self.cmp1, self.cmp2) as usize;
                self.emulator().output_ports[port] as u32
            }
            Parameter::WorkingMemory(index) => {
                let index = index.get(self.cmp1, self.cmp2) as usize;
                self.working_memory[index]
            }
            Parameter::ARAM(width, address) => {
                let address = address.get(self.cmp1, self.cmp2);
                let mut result = 0u32;
                for i in 0..width.width() {
                    result |= (self.emulator().read_u8(address + i) as u32) << (8 * i);
                }
                result
            }
            Parameter::IplROM(address) => {
                let address = address.get(self.cmp1, self.cmp2) as usize;
                self.emulator().ipl_rom[address] as u32
            }
            Parameter::DataArea(width,  ptr) => {
                let ptr = ptr.get(self.cmp1, self.cmp2) as usize;
                let end = ptr + width.width() as usize;

                let mut bytes = [0u8; 4];
                if end >= self.data_area.len() {
                    println!("[Script700] {}: warning: tried to read past end of data area", context);
                    if ptr < self.data_area.len() {
                        let capped_len = self.data_area.len() - ptr;
                        bytes[..capped_len].copy_from_slice(&self.data_area[ptr..]);
                    }
                } else {
                    bytes[..(end - ptr)].copy_from_slice(&self.data_area[ptr..end]);
                }

                u32::from_le_bytes(bytes)
            },
            Parameter::Label(label) => {
                let label = label.get(self.cmp1, self.cmp2) as u16;
                if let Some((is_data, ptr)) = self.labels.get(&label) {
                    if *is_data {
                        *ptr as u32
                    } else {
                        println!("[Script700] {}: warning: tried to reference script label :{} as a data label", context, label);
                        0
                    }
                } else {
                    println!("[Script700] {}: warning: tried to reference nonexistent data label :{}", context, label);
                    0
                }
            }
        }
    }

    fn write_parameter(&mut self, parameter: Parameter, value: u32, context: Arc<ScriptContext>) {
        match parameter {
            Parameter::Number(_) => {
                println!("[Script700] {}: warning: tried to write to numerical constant, ignoring", context);
            },
            Parameter::InputPort(port) => {
                let port = port.get(self.cmp1, self.cmp2) as usize;
                self.input_ports_buffer[port] = value as u8;
                if self.input_ports_unbuffered {
                    self.emulator().write_to_input_port(port, value as u8);
                }
            },
            Parameter::OutputPort(port) => {
                let port = port.get(self.cmp1, self.cmp2) as usize;
                self.emulator().output_ports[port] = value as u8;
            },
            Parameter::WorkingMemory(index) => {
                let index = index.get(self.cmp1, self.cmp2) as usize;
                self.working_memory[index] = value;
            },
            Parameter::ARAM(width, address) => {
                let address = address.get(self.cmp1, self.cmp2);
                let value = value.to_le_bytes();
                for i in 0..width.width() {
                    self.emulator().write_u8(address + i, value[i as usize]);
                }
            },
            Parameter::IplROM(_) => {
                println!("[Script700] {}: warning: tried to write to read-only IPL ROM, ignoring", context);
            },
            Parameter::DataArea(width, ptr) => {
                let ptr = ptr.get(self.cmp1, self.cmp2) as usize;
                let value = value.to_le_bytes();
                for i in 0..width.width() {
                    if ptr + i as usize >= self.data_area.len() {
                        println!("[Script700] {}: warning: tried to write past end of data area", context);
                        break;
                    }
                    self.data_area[ptr + i as usize] = value[i as usize];
                }
            },
            Parameter::Label(_) => {
                println!("[Script700] {}: warning: tried to write to data area label, you need to dereference it first", context);
            }
        }
    }

    pub fn trigger_port_event(&mut self, is_output: bool, port: u8) {
        debug_assert!(port < 4);
        if let Some(event) = self.wait_port_event {
            if event == (is_output, port) {
                self.wait_port_event = None;
                self.cmp1 = self.wait_port_cycles;
            }
        }
    }

    pub fn cycles_callback(&mut self, num_cycles: i32) {
        self.wait_cycles = self.wait_cycles.saturating_sub(2 * num_cycles as u32);
        if self.wait_port_event.is_some() {
            self.wait_port_cycles += num_cycles as u32;
        }

        while self.script_pc < self.script_ast.len() && self.wait_cycles == 0 && self.wait_port_event.is_none() {
            let (command, context) = self.script_ast[self.script_pc].clone();
            let mut runtime_error = false;

            match command {
                Command::Label { .. } => {},
                Command::Wait { cycles } => self.wait_cycles = self.read_parameter(cycles, context.clone()),
                Command::Move { src, dst } => {
                    let value = self.read_parameter(src, context.clone());
                    self.write_parameter(dst, value, context.clone());
                }
                Command::Compare { cmp1, cmp2 } => {
                    self.cmp1 = self.read_parameter(cmp1, context.clone());
                    self.cmp2 = self.read_parameter(cmp2, context.clone());
                }
                Command::Numeric { val, op, dst } => {
                    let x = self.read_parameter(dst, context.clone());
                    let y = self.read_parameter(val, context.clone());
                    match op {
                        Operation::Divide | Operation::UnsignedDivide => {
                            if y == 0 {
                                println!("[Script700] {}: error: division by 0", context);
                                runtime_error = true;
                            }
                        },

                        _ => ()
                    };
                    if !runtime_error {
                        let result = match op {
                            Operation::Add => (x as i32).wrapping_add(y as i32) as u32,
                            Operation::Subtract => (x as i32).wrapping_sub(y as i32) as u32,
                            Operation::Multiply => (x as i32).wrapping_mul(y as i32) as u32,
                            Operation::Divide => (x as i32).wrapping_div(y as i32) as u32,
                            Operation::UnsignedDivide => x.wrapping_div(y),
                            Operation::Remainder => (x as i32).wrapping_rem(y as i32) as u32,
                            Operation::Modulus => x.wrapping_rem_euclid(y),
                            Operation::And => x & y,
                            Operation::Or => x | y,
                            Operation::Xor => x ^ y,
                            Operation::ShiftLeft => x << y,
                            Operation::ShiftRightArithmetic => ((x as i32) >> (y as i32)) as u32,
                            Operation::ShiftRightLogical => x >> y,
                            Operation::Not => !y
                        };
                        self.write_parameter(dst, result, context.clone());
                    }
                }
                Command::Branch { condition, target } => {
                    let condition_satisfied = match condition {
                        Condition::Always => true,
                        Condition::Equal => self.cmp2 == self.cmp1,
                        Condition::NotEqual => self.cmp2 != self.cmp1,
                        Condition::GreaterOrEqual => (self.cmp2 as i32) >= (self.cmp1 as i32),
                        Condition::LessOrEqual => (self.cmp2 as i32) <= (self.cmp1 as i32),
                        Condition::GreaterThan => (self.cmp2 as i32) > (self.cmp1 as i32),
                        Condition::LessThan => (self.cmp2 as i32) < (self.cmp1 as i32),
                        Condition::CarryClear => self.cmp2 >= self.cmp1,
                        Condition::Lower => self.cmp2 <= self.cmp1,
                        Condition::Higher => self.cmp2 > self.cmp1,
                        Condition::CarrySet => self.cmp2 < self.cmp1
                    };

                    if condition_satisfied {
                        let target = self.read_parameter(target, context.clone()) as u16;
                        if let Some((is_data, ptr)) = self.labels.get(&target) {
                            if !is_data {
                                if self.call_stack_enabled {
                                    self.call_stack.push(self.script_pc);
                                }
                                self.script_pc = *ptr;
                            } else {
                                println!("[Script700] {}: error: tried to jump to data label :{}", context, target);
                                runtime_error = true;
                            }
                        } else {
                            println!("[Script700] {}: error: tried to jump to nonexistent label :{}", context, target);
                            runtime_error = true;
                        }
                    }
                }
                Command::Return => {
                    self.call_stack_enabled = true;
                    if let Some(pc) = self.call_stack.pop() {
                        self.script_pc = pc;
                    } else {
                        println!("[Script700] {}: error: tried to return but call stack is empty", context);
                        runtime_error = true;
                    }
                },
                Command::SetCallStack { enabled } => self.call_stack_enabled = enabled,
                Command::FlushInputPorts => {
                    self.input_ports_unbuffered = true;
                    self.emulator().write_u8(0xf5, self.input_ports_buffer[1]);
                    self.emulator().write_u8(0xf6, self.input_ports_buffer[2]);
                    self.emulator().write_u8(0xf7, self.input_ports_buffer[3]);
                    self.emulator().write_u8(0xf4, self.input_ports_buffer[0]);
                },
                Command::SetUnbufferedInputPorts { enabled } => self.input_ports_unbuffered = enabled,
                Command::WaitPort { port } => {
                    match port {
                        Parameter::InputPort(ParameterValue::Literal(port)) => {
                            self.wait_port_event = Some((false, port as u8));
                            self.wait_port_cycles = 0;
                        },
                        Parameter::OutputPort(ParameterValue::Literal(port)) => {
                            self.wait_port_event = Some((true, port as u8));
                            self.wait_port_cycles = 0;
                        },

                        _ => unreachable!()
                    }
                },
                Command::Quit => self.script_pc = self.script_ast.len(),
                Command::Nop => {
                    println!("[Script700] {}: error: nop", context);
                    runtime_error = true;
                },
                Command::Import { .. } => {}
            }

            // println!("[Script700] debug: executed @{} {:?}", self.script_pc, &command);
            // println!("            wait: {}, error: {}, r: {:?}, f: {:?}", self.wait_cycles, runtime_error, self.call_stack_enabled, self.input_ports_unbuffered);
            // println!("            cmp: {} {}, work: {:?}", self.cmp1, self.cmp2, self.working_memory);

            if runtime_error {
                loop {
                    self.script_pc += 1;

                    if self.script_pc >= self.script_ast.len() {
                        break;
                    }

                    if self.script_ast[self.script_pc].1 > context.clone() {
                        break;
                    }
                }
            } else {
                self.script_pc += 1;
            }
        }
    }
}
