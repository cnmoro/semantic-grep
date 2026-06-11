use std::io::{IsTerminal, Write};

use termcolor::{Color, ColorSpec, NoColor, StandardStream, WriteColor};

use crate::matcher::SearchMatch;

#[derive(Clone, Copy, PartialEq)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

pub type BoxWriter = Box<dyn WriteColor>;

pub struct Printer {
    writer: BoxWriter,
    show_filename: bool,
    show_line_number: bool,
    filename: Option<String>,
    count_mode: bool,
    files_with_matches_mode: bool,
}

impl Printer {
    pub fn new(
        color_when: ColorWhen,
        show_filename: bool,
        show_line_number: bool,
        count_mode: bool,
        files_with_matches_mode: bool,
    ) -> Self {
        let writer: BoxWriter = match color_when {
            ColorWhen::Never => Box::new(NoColor::new(std::io::stdout())),
            ColorWhen::Always => {
                Box::new(StandardStream::stdout(termcolor::ColorChoice::Always))
            }
            ColorWhen::Auto => {
                if std::io::stdout().is_terminal() {
                    Box::new(StandardStream::stdout(termcolor::ColorChoice::Always))
                } else {
                    Box::new(NoColor::new(std::io::stdout()))
                }
            }
        };
        Self {
            writer,
            show_filename,
            show_line_number,
            filename: None,
            count_mode,
            files_with_matches_mode,
        }
    }

    pub fn new_with_writer(
        writer: BoxWriter,
        show_filename: bool,
        show_line_number: bool,
    ) -> Self {
        Self {
            writer,
            show_filename,
            show_line_number,
            filename: None,
            count_mode: false,
            files_with_matches_mode: false,
        }
    }

    pub fn set_filename(&mut self, name: String) {
        self.filename = Some(name);
    }

    fn print_filename_prefix(&mut self) -> std::io::Result<()> {
        if let Some(ref name) = self.filename {
            if self.show_filename {
                self.writer.set_color(
                    ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true),
                )?;
                write!(self.writer, "{}:", name)?;
                self.writer.reset()?;
            }
        }
        Ok(())
    }

    fn print_line_number(&mut self, line_num: usize) -> std::io::Result<()> {
        if self.show_line_number {
            self.writer.set_color(
                ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true),
            )?;
            write!(self.writer, "{}:", line_num)?;
            self.writer.reset()?;
        }
        Ok(())
    }

    fn print_similarity(&mut self, sim: f64) -> std::io::Result<()> {
        self.writer.set_color(
            ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true),
        )?;
        write!(self.writer, "{:.4}:", sim)?;
        self.writer.reset()?;
        Ok(())
    }

    pub fn print_match(&mut self, m: &SearchMatch) -> std::io::Result<()> {
        if self.count_mode || self.files_with_matches_mode {
            return Ok(());
        }
        self.print_filename_prefix()?;
        self.print_line_number(m.line_number)?;
        if let Some(sim) = m.similarity {
            self.print_similarity(sim)?;
        }
        writeln!(self.writer, "{}", m.line)?;
        Ok(())
    }

    pub fn print_separator(&mut self) -> std::io::Result<()> {
        if self.show_filename {
            self.writer.set_color(
                ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true),
            )?;
            writeln!(self.writer, "--")?;
            self.writer.reset()?;
        }
        Ok(())
    }

    pub fn print_filename_only(&mut self, name: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{}", name)?;
        Ok(())
    }

    pub fn print_count(&mut self, count: usize, path: &str) -> std::io::Result<()> {
        if self.show_filename {
            write!(self.writer, "{}:", path)?;
        }
        writeln!(self.writer, "{}", count)?;
        Ok(())
    }

    pub fn set_count_mode(&mut self, v: bool) {
        self.count_mode = v;
    }

    pub fn set_files_with_matches_mode(&mut self, v: bool) {
        self.files_with_matches_mode = v;
    }
}

pub fn format_match_output(m: &SearchMatch, show_filename: bool, filename: Option<&str>, show_line_number: bool) -> String {
    let mut parts = Vec::new();
    if show_filename {
        if let Some(fname) = filename {
            parts.push(format!("{}:", fname));
        }
    }
    if show_line_number {
        parts.push(format!("{}:", m.line_number));
    }
    if let Some(sim) = m.similarity {
        parts.push(format!("{:.4}:", sim));
    }
    parts.push(m.line.clone());
    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_match(line_number: usize, line: &str, similarity: Option<f64>) -> SearchMatch {
        SearchMatch {
            line_number,
            line: line.to_string(),
            column: 1,
            similarity,
        }
    }

    #[test]
    fn test_format_match_output_basic() {
        let m = make_match(5, "hello world", None);
        let out = format_match_output(&m, false, None, true);
        assert_eq!(out, "5:hello world");
    }

    #[test]
    fn test_format_match_output_with_filename() {
        let m = make_match(3, "match line", None);
        let out = format_match_output(&m, true, Some("test.txt"), true);
        assert_eq!(out, "test.txt:3:match line");
    }

    #[test]
    fn test_format_match_output_with_similarity() {
        let m = make_match(1, "semantic line", Some(0.8571));
        let out = format_match_output(&m, false, None, true);
        assert!(out.contains("0.8571"));
        assert!(out.contains("semantic line"));
    }

    #[test]
    fn test_format_match_output_no_line_number() {
        let m = make_match(42, "no number", None);
        let out = format_match_output(&m, false, None, false);
        assert_eq!(out, "no number");
    }

    #[test]
    fn test_format_match_output_all_options() {
        let m = make_match(7, "full line", Some(0.95));
        let out = format_match_output(&m, true, Some("f.txt"), true);
        assert_eq!(out, "f.txt:7:0.9500:full line");
    }

    #[test]
    fn test_print_respects_count_mode() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), false, true);
        p.count_mode = true;
        let m = make_match(1, "hidden", None);
        p.print_match(&m).unwrap();
    }

    #[test]
    fn test_print_respects_files_with_matches_mode() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), false, true);
        p.files_with_matches_mode = true;
        let m = make_match(1, "hidden", None);
        p.print_match(&m).unwrap();
    }

    #[test]
    fn test_set_mode_flags() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), false, true);
        p.set_count_mode(true);
        assert!(p.count_mode);
        p.set_files_with_matches_mode(true);
        assert!(p.files_with_matches_mode);
    }

    #[test]
    fn test_set_filename() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), true, false);
        p.set_filename("foo.txt".into());
        assert_eq!(p.filename.as_deref(), Some("foo.txt"));
    }

    #[test]
    fn test_print_match_no_crash() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), false, true);
        let m = make_match(1, "test", None);
        p.print_match(&m).unwrap();
    }

    #[test]
    fn test_print_filename_only_no_crash() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), true, true);
        p.print_filename_only("test.txt").unwrap();
    }

    #[test]
    fn test_print_count_no_crash() {
        let buf: Vec<u8> = Vec::new();
        let mut p = Printer::new_with_writer(Box::new(NoColor::new(buf)), true, true);
        p.print_count(5, "test.txt").unwrap();
    }
}
