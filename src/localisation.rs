use chumsky::Parser;
use chumsky::prelude::*;
use chumsky::text::{Character, ident, newline, whitespace};

pub fn localisation_ident<C: Character, E: chumsky::Error<C>>() -> impl Parser<C, C::Collection, Error = E> + Copy + Clone {
    filter(|c: &C| c.to_char() != '\n' && c.to_char() != '"' && c.to_char() != ' ')
        .map(Some)
        .chain::<C, Vec<_>, _>(
            filter(|c: &C| c.to_char() != '\n' && c.to_char() != '"' && c.to_char() != ' ').repeated(),
        )
        .collect()
}

pub fn parser() -> impl Parser<char, (String, Vec<(String, String)>), Error = Simple<char>> {
    let comment = just::<_, _, Simple<char>>('#').then(take_until(newline())).padded().ignored();

    let first = just("l_").ignore_then(ident()).then_ignore(just(':')).padded().padded_by(comment.padded().repeated());

    let escape = just('\\').ignore_then(
        just('\\')
            .or(just('/'))
            .or(just('"'))
            .or(just('b').to('\x08'))
            .or(just('f').to('\x0C'))
            .or(just('n').to('\n'))
            .or(just('r').to('\r'))
            .or(just('t').to('\t'))
            .or(just('u').ignore_then(
                filter(|c: &char| c.is_digit(16))
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .validate(|digits, span, emit| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap())
                            .unwrap_or_else(|| {
                                emit(Simple::custom(span, "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            })
                    }),
            )),
    );

    let string = just('"')
        .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .labelled("string");


    let pair = text::whitespace()
        .ignore_then(localisation_ident())
        .then_ignore(just(':').ignored().or(whitespace().ignored()))
        .padded()
        .then(string)
        .then_ignore(newline())
        .padded_by(comment.repeated());

    first.then(pair.repeated())

}