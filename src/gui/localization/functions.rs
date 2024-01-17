use fluent::{FluentArgs, FluentValue};

macro_rules! fluent_fn_arg {
    ($args: ident [ $key: expr ]) => {{
        match $args.get($key).cloned() {
            Some(v) => v,
            None => return FluentValue::Error
        }
    }};
    ($args: ident [ $key: expr ] -> $t: tt) => {{
        match $args.get($key).cloned() {
            Some(FluentValue::$t(v)) => v,
            _ => return FluentValue::Error
        }
    }};
}

pub fn fluent_fn_number<'a>(args: &[FluentValue<'a>], named_args: &FluentArgs<'_>) -> FluentValue<'a> {
    let num_val = match fluent_fn_arg!(args[0]) {
        FluentValue::String(s) => FluentValue::try_number(s),
        other => other
    };
    let mut num = match num_val {
        FluentValue::Number(n) => n,
        other => return other
    };

    num.options.merge(named_args);

    FluentValue::Number(num)
}
