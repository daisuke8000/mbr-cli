pub fn print_verbose(verbose: bool, msg: &str) {
    if verbose {
        println!("Verbose: {}", msg);
    }
}

pub fn print_verbose_with_prefix(verbose: bool, prefix: &str, msg: &str) {
    if verbose {
        println!("{}: {}", prefix, msg);
    }
}

pub fn log_error(msg: &str) {
    eprintln!("Error: {}", msg);
}

pub fn log_warning(msg: &str) {
    eprintln!("Warning: {}", msg);
}

pub fn log_info(msg: &str) {
    println!("Info: {}", msg);
}

pub struct VerboseLogger {
    enabled: bool,
}

impl VerboseLogger {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn log(&self, msg: &str) {
        print_verbose(self.enabled, msg);
    }

    pub fn log_with_prefix(&self, prefix: &str, msg: &str) {
        print_verbose_with_prefix(self.enabled, prefix, msg);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
