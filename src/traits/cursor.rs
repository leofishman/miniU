use std::io::Write;

pub enum SpinnerType {
    Moon,
    Clock,
    Dots,
}

pub struct CursorBusy {
    frames: Vec<&'static str>,
    current_frame: usize,
}

pub trait Cursor {
    fn new(spinner_type: SpinnerType) -> Self;
    fn tick(&mut self, message: &str);
}

impl Cursor for CursorBusy {
    fn new(spinner_type: SpinnerType) -> Self {
        let frames = match spinner_type {
            SpinnerType::Moon => vec!["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"],
            SpinnerType::Clock => vec!["⏳", "⏳", "⌛"],
            SpinnerType::Dots => vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        };
        Self { frames, current_frame: 0 }
    }

    fn tick(&mut self, message: &str) {
        let frame = self.frames[self.current_frame];
        // \r nos devuelve al inicio de la línea
        print!("\r{} {} ", frame, message);
        std::io::stdout().flush().unwrap();
        
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }
}