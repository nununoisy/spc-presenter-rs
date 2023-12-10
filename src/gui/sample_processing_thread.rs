use std::collections::HashMap;
use anyhow::{Error, anyhow};
use std::thread;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use crate::sample_processing::{SampleProcessor, SampleProcessorProgress, SampleData};

#[derive(Clone)]
pub enum SampleProcessingThreadRequest {
    StartProcessing(String),
    CancelProcessing,
    Terminate
}

pub enum SampleProcessingThreadMessage {
    Error(Error),
    ProcessingStarting,
    ProcessingProgress(SampleProcessorProgress),
    ProcessingComplete(HashMap<u8, SampleData>),
    ProcessingCancelled
}

macro_rules! rt_unwrap {
    ($v: expr, $cb: tt, $lbl: tt) => {
        match $v {
            Ok(v) => v,
            Err(e) => {
                $cb(SampleProcessingThreadMessage::Error(e));
                continue $lbl;
            }
        }
    };
}

pub fn sample_processing_thread<F>(cb: F) -> (thread::JoinHandle<()>, mpsc::Sender<SampleProcessingThreadRequest>)
    where
        F: Fn(SampleProcessingThreadMessage) + Send + 'static
{
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        println!("Sample processing thread started");

        'main: loop {
            let spc_path = match rx.recv().unwrap() {
                SampleProcessingThreadRequest::StartProcessing(spc_path) => spc_path,
                SampleProcessingThreadRequest::CancelProcessing => continue 'main,
                SampleProcessingThreadRequest::Terminate => break 'main
            };
            cb(SampleProcessingThreadMessage::ProcessingStarting);

            let mut sample_processor = rt_unwrap!(SampleProcessor::from_spc(spc_path), cb, 'main);

            let mut last_progress_timestamp = Instant::now();
            // Janky way to force an update
            last_progress_timestamp.checked_sub(Duration::from_secs(2));

            'processing: loop {
                match rx.try_recv() {
                    Ok(SampleProcessingThreadRequest::StartProcessing(_)) => {
                        cb(SampleProcessingThreadMessage::Error(anyhow!("Cannot start processing another SPC without cancelling first.")));
                    },
                    Ok(SampleProcessingThreadRequest::CancelProcessing) => {
                        cb(SampleProcessingThreadMessage::ProcessingCancelled);
                        break 'processing;
                    },
                    Ok(SampleProcessingThreadRequest::Terminate) => break 'main,
                    _ => ()
                }

                match rt_unwrap!(sample_processor.step(), cb, 'main) {
                    SampleProcessorProgress::Finished => break 'processing,
                    progress => {
                        if last_progress_timestamp.elapsed().as_secs_f64() >= 0.01 {
                            last_progress_timestamp = Instant::now();
                            cb(SampleProcessingThreadMessage::ProcessingProgress(progress));
                        }
                    }
                }
            }

            let sample_data = sample_processor.finish();
            cb(SampleProcessingThreadMessage::ProcessingComplete(sample_data));
        }
    });
    (handle, tx)
}
