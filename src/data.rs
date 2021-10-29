use super::parse::{Block, BulletItem, Span, PictureDirective, ParsePicture};
use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct Presentation {
    pub(crate) title: Title,
    pub(crate) author: String,
    //author: String,
    pub(crate) slides: Vec<Slide>,
}

#[derive(Debug)]
pub(crate) struct Slide {
    pub(crate) title: Title,
    pub(crate) contents: ContentOptions,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ContentOptions {
    OnlyText(Vec<Block>),
    OnlyPicture(Picture),
    TextAndPicture(Vec<Block>, Picture),
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Picture {
    path: String,
    caption: Option<String>,
    directive: Option<PictureDirective>
}

impl Picture {
    fn to_latex_picture(&self, is_split: bool) -> LatexPicture {
        LatexPicture {
            picture: &self,
            is_split,
        }
    }
}

impl From<ParsePicture> for Picture {
    fn from(x: ParsePicture) -> Self {
        let ParsePicture {path, caption, directive} = x;
        Self{path, caption, directive}
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Code {
    language: String,
    text: String,
}

impl Code {
    pub(crate) fn new(text: String, language: String) -> Self {
        Code { language, text }
    }
}

impl<'a> Latex for Code {
    fn to_latex(&self, buffer: &mut String) {
        buffer.push_str(r#"\begin{lstlisting}[language="#);
        buffer.push_str(&self.language);
        buffer.push_str("]\n");

        buffer.push_str(&self.text);

        buffer.push_str("\\end{lstlisting}")
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct LatexPicture<'a> {
    picture: &'a Picture,
    is_split: bool,
}

impl<'a> LatexPicture<'a> {
    fn get_path(&self) -> String {
        self.picture.path.clone()
    }

    fn caption(&self) -> Option<&str> {
        self.picture.caption.as_ref().map(|x| x.as_str())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Title {
    pub(crate) title: Vec<Span>,
}

impl From<Vec<Span>> for Title {
    fn from(title: Vec<Span>) -> Self {
        Self { title }
    }
}

pub(crate) trait Latex {
    fn to_latex(&self, buffer: &mut String);
}

impl Latex for Title {
    fn to_latex(&self, buffer: &mut String) {
        self.title.to_latex(buffer);
    }
}

impl<'a> Latex for LatexPicture<'a> {
    fn to_latex(&self, buffer: &mut String) {
        let path = self.get_path();
        buffer.push_str(
            r#"
    \begin{figure}
        \centering"#,
        );

        if self.is_split {
            buffer.push_str(
                r#"
        \includegraphics[width=\textwidth]{"#,
            );
        } else {
            buffer.push_str(
                r#"
        \includegraphics[width=0.9\paperwidth,height=0.7\paperheight,keepaspectratio]{"#,
            );
        }

        buffer.push_str(&path);
        buffer.push_str(r#"}"#);

        // only add a caption if we have one
        if let Some(caption) = self.caption() {
            buffer.push_str(r#"\caption{"#);
            buffer.push_str(caption);
            buffer.push_str( r#"}"#);
        }
        
        buffer.push_str(r#"\end{figure}"#)
    }
}

impl Latex for ContentOptions {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            ContentOptions::OnlyText(content_list) => {
                for c in content_list {
                    c.to_latex(buffer);
                    buffer.push_str("\n\n")
                }
            }
            ContentOptions::OnlyPicture(picture) => {
                picture.to_latex_picture(false).to_latex(buffer);
            }
            ContentOptions::TextAndPicture(content, picture) => {

                match picture.directive {
                    Some(PictureDirective::Vertical) => {

                        for c in content {
                            c.to_latex(buffer)
                        }

                        picture.to_latex_picture(false).to_latex(buffer);
                    }
                    None => {
                        buffer.push_str("\t\\begin{minipage}{0.4\\textwidth}\n");

                        for c in content {
                            c.to_latex(buffer)
                        }

                        buffer.push_str("\n\t\\end{minipage}%\n");
                        buffer.push_str("\t\\hfill\n");
                        buffer.push_str("\t\\begin{minipage}{0.55\\textwidth}\n");

                        picture.to_latex_picture(true).to_latex(buffer);

                        buffer.push_str("\t\\end{minipage}\n");
                    }
                }
            }
        }
    }
}

impl Latex for Block {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            Block::Paragraph(spans) => spans.to_latex(buffer),
            Block::BulletedList(span_spans) => span_spans.to_latex(buffer),
            Block::Picture(_) => {
                panic!("pictures should be removed from blocks prior to processing")
            }
            Block::Directive(_) => {
                panic!("directives should have been remove from the presentation as a postprocess step")
            }
            Block::Code(code) => code.to_latex(buffer),
        }
    }
}

impl Latex for Vec<BulletItem> {
    fn to_latex(&self, buffer: &mut String) {
        buffer.push_str("\n\\begin{itemize}\n");

        for item in self.iter() {
            item.to_latex(buffer)
        }

        buffer.push_str("\n\\end{itemize}\n");
    }
}

impl Latex for BulletItem {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            BulletItem::Single(spans) => {
                buffer.push_str("\\item ");
                spans.to_latex(buffer);
                buffer.push('\n');
            }
            BulletItem::Nested(item_list) => item_list.to_latex(buffer),
        }
    }
}

impl Latex for Vec<Span> {
    fn to_latex(&self, buffer: &mut String) {
        for span in self.iter() {
            span.to_latex(buffer);
        }
    }
}

impl Latex for Span {
    fn to_latex(&self, buffer: &mut String) {
        match self {
            Span::Bold(s) => wrap_text(buffer, "\\textbf{", &s, "}"),
            Span::Strikethrough(s) => wrap_text(buffer, "\\sout{", &s, "}"),
            Span::Italics(s) => wrap_text(buffer, "\\emph{", &s, "}"),
            Span::Text(s) => wrap_text(buffer, "", &s, ""),
            Span::Equation(s) => wrap_text(buffer, "$$", &s, "$$"),
        }
    }
}

fn wrap_text(buffer: &mut String, start: &'static str, inner: &str, end: &'static str) {
    buffer.push_str(start);
    buffer.push_str(inner);
    buffer.push_str(end);
}
