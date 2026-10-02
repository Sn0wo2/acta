use anstyle_parse::Perform;

const BOLD: u8 = 1;
const DIM: u8 = 2;
const ITALIC: u8 = 4;
const UNDERLINE: u8 = 8;
const STRIKE: u8 = 16;

#[derive(Default)]
struct ANSI2CSS {
    segments: Vec<(String, String)>,
    text: String,
    fg: Option<(u8, u8, u8)>,
    bg: Option<(u8, u8, u8)>,
    effects: u8,
}

impl ANSI2CSS {
    fn flush(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let css = self.css();
        let text = std::mem::take(&mut self.text);
        self.segments.push((css, text));
    }

    fn css(&self) -> String {
        let mut out = String::new();
        if let Some((r, g, b)) = self.fg {
            out.push_str(&format!("color:rgb({r},{g},{b});"));
        }
        if let Some((r, g, b)) = self.bg {
            out.push_str(&format!("background-color:rgb({r},{g},{b});"));
        }
        if self.effects & BOLD != 0 {
            out.push_str("font-weight:bold;");
        }
        if self.effects & ITALIC != 0 {
            out.push_str("font-style:italic;");
        }
        if self.effects & UNDERLINE != 0 {
            out.push_str("text-decoration:underline;");
        }
        if self.effects & STRIKE != 0 {
            out.push_str("text-decoration:line-through;");
        }
        if self.effects & DIM != 0 {
            out.push_str("opacity:0.7;");
        }
        out
    }

    fn apply_sgr(&mut self, nums: &[u16]) {
        let mut i = 0;
        while let Some(&n) = nums.get(i) {
            match n {
                0 => {
                    self.fg = None;
                    self.bg = None;
                    self.effects = 0;
                }
                1 => self.effects |= BOLD,
                2 => self.effects |= DIM,
                3 => self.effects |= ITALIC,
                4 => self.effects |= UNDERLINE,
                9 => self.effects |= STRIKE,
                22 => self.effects &= !(BOLD | DIM),
                23 => self.effects &= !ITALIC,
                24 => self.effects &= !UNDERLINE,
                29 => self.effects &= !STRIKE,
                39 => self.fg = None,
                49 => self.bg = None,
                30..=37 => self.fg = Some(ansi_colours::rgb_from_ansi256((n - 30) as u8)),
                40..=47 => self.bg = Some(ansi_colours::rgb_from_ansi256((n - 40) as u8)),
                90..=97 => self.fg = Some(ansi_colours::rgb_from_ansi256((n - 90 + 8) as u8)),
                100..=107 => self.bg = Some(ansi_colours::rgb_from_ansi256((n - 100 + 8) as u8)),
                38 | 48 => {
                    let slot = if n == 38 { &mut self.fg } else { &mut self.bg };
                    match nums.get(i + 1..) {
                        Some([2, r, g, b, ..]) => {
                            *slot = Some((*r as u8, *g as u8, *b as u8));
                            i += 4;
                        }
                        Some([5, v, ..]) => {
                            *slot = Some(ansi_colours::rgb_from_ansi256(*v as u8));
                            i += 2;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

impl Perform for ANSI2CSS {
    fn print(&mut self, c: char) {
        self.text.push(c);
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\n' {
            self.text.push('\n');
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &anstyle_parse::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: u8,
    ) {
        if action != b'm' {
            // byte 'm', example: \x1b[31m
            return;
        }
        self.flush();
        self.apply_sgr(&params.iter().flatten().copied().collect::<Vec<u16>>());
    }
}

#[allow(clippy::single_call_fn)]
pub(crate) fn ansi_to_css(msg: &str) -> (String, Vec<String>) {
    let mut conv = ANSI2CSS {
        text: String::with_capacity(msg.len()),
        ..Default::default()
    };
    let mut parser = anstyle_parse::Parser::<anstyle_parse::Utf8Parser>::new();
    for byte in msg.as_bytes() {
        parser.advance(&mut conv, *byte);
    }
    conv.flush();

    if !conv.segments.iter().any(|(css, _)| !css.is_empty()) {
        return (
            conv.segments
                .into_iter()
                .map(|(_, text)| text)
                .collect::<String>(),
            Vec::new(),
        );
    }

    let mut fmt = String::with_capacity(msg.len());
    for (_, text) in &conv.segments {
        fmt.push_str("%c");
        fmt.push_str(text);
    }
    (fmt, conv.segments.into_iter().map(|(css, _)| css).collect())
}
