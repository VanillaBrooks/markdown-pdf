use super::data::{ContentOptions, Picture, Presentation, Slide, Title};
use super::parse::{Span, Block, Document, ParsedTitle, ParsedSlide, Directive, PictureDirective};

pub(crate) fn postprocess(presentation: Document) -> Presentation {
    let slides = presentation
        .slides
        .into_iter()
        .map(process_slide)
        .flatten()
        .collect::<Vec<_>>();

    Presentation {
        title: presentation.first.title.into(),
        author: presentation.first.author,
        slides,
    }
}

fn process_slide(slide: ParsedSlide) -> Vec<Slide> {
    // first organize the slides based on the directives
    let slides = text_directive_handler(slide.contents);

    // then map each of the slides into either plain text or a text with picture
    slides.into_iter()
        .map(to_content_options)
        .map(|contents| Slide{ title: slide.title.clone().into(), contents })
        .collect()

    // // if we know that we have a slide that only contains a picture
    // // with potentially some directives...
    // let picture_or_directive =
    //     slide.contents.iter().map(|x| matches!(x, Block::Directive(_)) || matches!(x, Block::Picture(_)))
    //     .find(|x| *x == false);

    // if picture_or_directive.is_none() {
    //     return picture_with_directives(slide.title, slide.contents)
    // }

    // // check to see if the slide only contains text w/ some directives
    // // and there are no pictures
    // let only_text =
    //     slide.contents.iter().map(|x| matches!(x, Block::Directive(_)) && !matches!(x, Block::Picture(_)))
    //     .find(|x| *x == false);

    // if only_text.is_none() {
    //     return only_text_with_directives(slide.title, slide.contents)
    // }

    // // otherwise, if we are here then we know there is a mixture of text and pictures
    // text_with_pictures(slide.title, slide.contents)

}

/// this function cannot be called with a picture
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

fn text_with_pictures(title: Vec<Span>, contents: Vec<Block>) -> Vec<Slide> {
    // first organize the slides based on the directives
    let slides = text_directive_handler(contents);

    // then map each of the slides into either plain text or a text with picture
    slides.into_iter()
        .map(to_content_options)
        .map(|contents| Slide{ title: title.clone().into(), contents })
        .collect()
}

fn only_text_with_directives(title: Vec<Span>, contents: Vec<Block>) -> Vec<Slide> {
    todo!()
}

// we have a slide that just contains some directives, and some pictures
fn picture_with_directives(title: Vec<Span>, contents: Vec<Block>) -> Vec<Slide> {
    todo!()
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
