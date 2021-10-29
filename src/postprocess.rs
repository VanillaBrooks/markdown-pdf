use super::data::{ContentOptions, Picture, Presentation, Slide, Title};
use super::parse::{Span, Block, Document, ParsedTitle, ParsedSlide, Directive, PictureDirective};

pub(crate) fn postprocess(presentation: Document, ignore_newslide:bool) -> Presentation {
    let slides = presentation
        .slides
        .into_iter()
        .map(|x| process_slide(x, ignore_newslide))
        .flatten()
        .collect::<Vec<_>>();

    Presentation {
        title: presentation.first.title.into(),
        author: presentation.first.author,
        slides,
    }
}

fn process_slide(slide: ParsedSlide, ignore_newslide: bool) -> Vec<Slide> {
    // first organize the slides based on the directives
    let slides = text_directive_handler(slide.contents, ignore_newslide);

    // then map each of the slides into either plain text or a text with picture
    slides.into_iter()
        .map(to_content_options)
        .map(|contents| Slide{ title: slide.title.clone().into(), contents })
        .collect()
}

/// this function cannot be called with a picture
fn text_directive_handler(contents: Vec<Block>, ignore_newslide: bool) -> Vec<Vec<Block>> {
    let mut current_slide_contents = Vec::new();
    let mut out = Vec::new();

    for block in contents {
        if let Block::Directive(direc) = block {
            // handle the directive
            match direc {
                // we need to create a new slide from these contents
                Directive::NewSlide => {

                    // as long as we dont have a CLI argument to ignore these directives...
                    if !ignore_newslide {
                        // copy the current contents, save the slide, and then copy the new contents
                        // back
                        let tmp_contents = current_slide_contents.clone();

                        out.push(current_slide_contents);

                        current_slide_contents = tmp_contents;
                    }
                }
            }
        } else {
            // we have no new directives, just add the
            // block to the current slide
            current_slide_contents.push(block)
        }
    }

    out.push(current_slide_contents);

    out
}

fn is_only_text(current_contents: &[Block]) -> bool {
    for c in current_contents {
        if matches!(c, Block::Picture(_)) {
            return false
        }
    }
    true
}

fn is_only_picture(current_contents: &[Block]) -> bool {
    for c in current_contents {
        if !matches!(c, Block::Picture(_)) {
            return false
        }
    }
    true
}

// convert a slide into its renderable formk
fn to_content_options(slide_content: Vec<Block>) -> ContentOptions {
    if is_only_text(&slide_content) {
        ContentOptions::OnlyText(slide_content)
    } else if is_only_picture(&slide_content) {

        // grab the first picture in the slides
        let picture = match slide_content.into_iter().next().unwrap(){
            Block::Picture(x) => x,
            _ => unreachable!()
        };

        ContentOptions::OnlyPicture(picture.into())
    } else {
        let (picture, text) : (Vec<_>, Vec<_>) = slide_content.into_iter() .partition(|x| matches!(x, Block::Picture(_)));

        let picture = match picture.into_iter().next().unwrap(){
            Block::Picture(x) => x,
            _ => unreachable!()
        };

        ContentOptions::TextAndPicture(text, picture.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Span;

    fn spans_from_text(text: &str) -> Vec<Span> {
        vec![Span::Text(text.to_string())]
    }

    fn paragraph(text: &str) -> Block {
        Block::Paragraph(spans_from_text(text))
    }

    #[test]
    fn simple_newslide_directive() {
        let blocks = vec![
            paragraph("1"),
            Block::Directive(Directive::NewSlide),
            paragraph("2"),
        ];

        let out_blocks = text_directive_handler(blocks);

        let slide_1 = vec![paragraph("1")];

        let slide_2 = vec![paragraph("1"), paragraph("2")];

        assert_eq!(out_blocks[0], slide_1);
        assert_eq!(out_blocks[1], slide_2);
    }
}
