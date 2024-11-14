use gecko_profile::{Frame, ProfileBuilder, ThreadBuilder};
use goblin::elf::{sym::STT_FUNC, Elf};
use indicatif::{ProgressBar, ProgressStyle};
use rustc_demangle::demangle;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ProfilerError {
    #[error("Failed to read ELF file {}", .0)]
    Io(#[from] std::io::Error),
    #[error("Failed to parse ELF file {}", .0)]
    Elf(#[from] goblin::error::Error),
    #[error("Failed to serialize samples {}", .0)]
    Serde(#[from] serde_json::Error),
}

pub struct Profiler {
    sample_rate: u64,
    start_lookup: HashMap<u64, usize>,
    function_ranges: Vec<(u64, u64, Arc<str>)>,

    function_stack: Vec<Arc<str>>,
    function_stack_indices: Vec<usize>,
    function_stack_ranges: Vec<(u64, u64)>,
    current_function_range: (u64, u64),

    samples: Vec<Sample>,
}

struct Sample {
    stack: Vec<Arc<str>>,
}

impl Profiler {
    pub(super) fn new(elf_bytes: &[u8], sample_rate: u64) -> Result<Self, ProfilerError> {
        let elf = Elf::parse(elf_bytes)?;

        let mut start_lookup = HashMap::new();
        let mut function_ranges = Vec::new();

        for sym in &elf.syms {
            if sym.st_type() == STT_FUNC {
                let name = elf.strtab.get_at(sym.st_name).unwrap_or("");
                let demangled_name = demangle(name);
                let size = sym.st_size;
                let start_address = sym.st_value;
                let end_address = start_address + size - 4;
                let demangled: Arc<str> = demangled_name.to_string().into();

                let index = function_ranges.len();
                function_ranges.push((start_address, end_address, demangled));
                start_lookup.insert(start_address, index);
            }
        }

        Ok(Self {
            sample_rate,
            samples: Vec::new(),
            start_lookup,
            function_ranges,
            function_stack: Vec::new(),
            function_stack_indices: Vec::new(),
            function_stack_ranges: Vec::new(),
            current_function_range: (0, 0),
        })
    }

    pub(super) fn record(&mut self, pc: u64) {
        // We are still in the current function.
        if pc > self.current_function_range.0 && pc <= self.current_function_range.1 {
            if pc % self.sample_rate == 0 {
                self.samples.push(Sample { stack: self.function_stack.clone() });
            }

            return;
        }

        // Jump to a new function (or the same one).
        if let Some(f) = self.start_lookup.get(&pc) {
            // Jump to a new function (not recursive).
            if !self.function_stack_indices.contains(f) {
                self.function_stack_indices.push(*f);
                let (start, end, name) = self.function_ranges.get(*f).unwrap();
                self.current_function_range = (*start, *end);
                self.function_stack_ranges.push((*start, *end));
                self.function_stack.push(name.clone());
            }
        } else {
            // This means pc now points to an instruction that is
            //
            // 1. not in the current function's range
            // 2. not a new function call
            //
            // We now account for a new possibility where we're returning to a function in the
            // stack this need not be the immediate parent and can be any of the existing
            // functions in the stack due to some optimizations that the compiler can make.
            let mut unwind_point = 0;
            let mut unwind_found = false;
            for (c, (s, e)) in self.function_stack_ranges.iter().enumerate() {
                if pc > *s && pc <= *e {
                    unwind_point = c;
                    unwind_found = true;
                    break;
                }
            }

            // Unwinding until the parent.
            if unwind_found {
                self.function_stack.truncate(unwind_point + 1);
                self.function_stack_ranges.truncate(unwind_point + 1);
                self.function_stack_indices.truncate(unwind_point + 1);
            }

            // If no unwind point has been found, that means we jumped to some random location
            // so we'll just increment the counts for everything in the stack.
        }

        if pc % self.sample_rate == 0 {
            self.samples.push(Sample { stack: self.function_stack.clone() });
        }
    }

    pub(super) fn write(mut self, writer: impl std::io::Write) -> Result<(), ProfilerError> {
        self.check_samples();

        let start_time = std::time::Instant::now();
        let mut profile_builder = ProfileBuilder::new(
            start_time,
            std::time::SystemTime::now(),
            "SP1 ZKVM",
            0,
            std::time::Duration::from_micros(1),
        );

        let pb = ProgressBar::new(self.samples.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg} \n {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        pb.set_message("Creating profile");

        let mut thread_builder = ThreadBuilder::new(1, 0, start_time, false, false);

        let mut last_known_time = std::time::Instant::now();
        for sample in self.samples.drain(..) {
            pb.inc(1);
            let mut frames = Vec::new();
            for frame in sample.stack {
                frames.push(Frame::Label(thread_builder.intern_string(&frame)));
            }

            thread_builder.add_sample(
                last_known_time,
                frames.into_iter(),
                // We don't have a way to know the duration of each sample, so we just use 1us for
                // all instructions
                std::time::Duration::from_micros(self.sample_rate),
            );

            last_known_time += std::time::Duration::from_micros(self.sample_rate);
        }

        pb.finish();

        profile_builder.add_thread(thread_builder);

        println!("Writing profile, this can take awhile");
        serde_json::to_writer(writer, &profile_builder.to_serializable())?;
        println!("Profile written successfully");

        Ok(())
    }

    fn check_samples(&self) {
        let main_count =
            self.samples.iter().filter(|s| s.stack.iter().any(|f| &**f == "main")).count();

        #[allow(clippy::cast_precision_loss)]
        let main_ratio = main_count as f64 / self.samples.len() as f64;
        if main_ratio < 0.9 {
            eprintln!("Warning: This trace appears to be invalid. The `main` function is present in only {:.2}% of the samples, this is likely caused by the using the wrong Elf file", main_ratio * 100.0);
        }
    }
}
