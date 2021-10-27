use super::data::{ContentOptions, Picture, Presentation, Slide, Title};
use super::parse::{Block, Directive};

pub(crate) fn postprocess(mut presentation: Presentation) -> Presentation {
    presentation.slides = 
    presentation
        .slides
        .into_iter()
        .map(|slide| {
            match slide.contents {
                ContentOptions::OnlyText(text) => text_slide_directives(slide.title, text),
                ContentOptions::OnlyPicture(picture) => {
                    vec![Slide {
                        title: slide.title,
                        contents: ContentOptions::OnlyPicture(picture),
                    }]
                }
                ContentOptions::TextAndPicture(text, picture) => {
                    text_picture_handler(slide.title, text, picture)
                }
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    presentation
}

// takes in the contents of a slide and handles any directives for layout that may be present
// returns a vector of slides
fn text_slide_directives(title: Title, contents: Vec<Block>) -> Vec<Slide> {
    text_directive_handler(contents)
        .into_iter()
        .map(|contents| Slide {
            title: title.clone(),
            contents: ContentOptions::OnlyText(contents),
        })
        .collect()
}

fn text_directive_handler(contents: Vec<Block>) -> Vec<Vec<Block>> {
    let mut current_slide_contents = Vec::new();
    let mut out = Vec::new();

    for block in contents {
        if let Block::Directive(direc) = block {
            // handle the directive
            match direc {
                // we need to create a new slide from these contents
                Directive::NewSlide => {
                    // copy the current contents, save the slide, and then copy the new contents
                    // back
                    let tmp_contents = current_slide_contents.clone();

                    out.push(current_slide_contents);

                    current_slide_contents = tmp_contents;
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

// takes in the contents of a slide and handles any directives for layout that may be present
// returns a vector of slides
fn text_picture_handler(title: Title, contents: Vec<Block>, picture: Picture) -> Vec<Slide> {
    text_directive_handler(contents)
        .into_iter()
        .map(|contents| {
            Slide {
                title: title.clone(),
                contents: ContentOptions::TextAndPicture(contents, picture.clone()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests{
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

        let slide_1 = vec![
            paragraph("1"),
        ];

        let slide_2 = vec![
            paragraph("1"),
            paragraph("2"),
        ];

        assert_eq!(out_blocks[0], slide_1);
        assert_eq!(out_blocks[1], slide_2);

    }
}

