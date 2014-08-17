use parser::{MarkdownParser, Cursor, PhantomMark, ParseResult, Success, End, NoParse};
use tokens::*;
use util::CharOps;

use self::emphasis::EmphasisParser;
use self::escape::EscapeParser;

mod emphasis;
mod escape;

pub trait InlineParser {
    fn parse_inline(&self) -> Text;
}

struct InlineParsingState<'b, 'a> {
    tokens: Vec<Inline>,
    cur: &'b Cursor<'a>,
    pm: PhantomMark,
    pm_last: PhantomMark
}

impl<'b, 'a> InlineParsingState<'b, 'a> {
    #[inline]
    fn update(&mut self) {
        self.pm = self.cur.phantom_mark();
        self.pm_last = self.pm;
    }

    fn push_token(&mut self, token: Inline) {
        fn is_chunk(token: Option<&Inline>) -> bool {
            match token {
                Some(&Chunk(_)) => true,
                _ => false
            }
        }

        match token {
            Chunk(buf0) => if is_chunk(self.tokens.last()) {
                match self.tokens.mut_last().unwrap() {
                    &Chunk(ref mut buf) => buf.push_str(buf0.as_slice()),
                    _ => unreachable!()
                }
            } else {
                self.tokens.push(Chunk(buf0))
            },
            token => self.tokens.push(token)
        }
    }

    fn push_chunk(&mut self) {
        {
            debug!(">> pushing chunk from {} to {}", self.pm.pos, self.pm_last.pos);
            let slice = self.cur.slice(self.pm, self.pm_last);
            debug!(">> chunk: {}", ::std::str::from_utf8(slice).unwrap());
            if slice.is_empty() { return; }

            let chunk = slice.to_vec();
            // TODO: handle UTF-8 decoding error
            self.tokens.push(Chunk(String::from_utf8(chunk).unwrap()));
        }

        self.update();
    }

    #[inline]
    fn advance(&mut self) {
        self.pm_last = self.cur.phantom_mark();
        debug!(">> advanced to {}", self.pm_last.pos);
    }
}


impl<'a> InlineParser for MarkdownParser<'a> {
    fn parse_inline(&self) -> Text {
        debug!(">> parsing inline");

        let mut s = InlineParsingState {
            tokens: Vec::new(),
            cur: &self.cur,
            pm: self.cur.phantom_mark(),
            pm_last: self.cur.phantom_mark()
        };

        loop {
            debug!(">> cursor positon: {}", self.cur.pos);
            let c = opt_break!(self.cur.next_byte());
            match c {
                b'\\' => match break_on_end!(self.parse_escape()).unwrap() {
                    Some(token) => {
                        s.push_chunk();
                        s.push_token(token);
                        s.update();
                    }
                    None => s.advance()
                },

                c if c.is_emphasis() || c.is_code() => {
                    debug!(">> encountered emphasis");
                    s.push_chunk();

                    // one or two emphasis characters
                    let mut n = 1;
                    if break_on_end!(self.try_read_char(c)).is_success() {
                        n += 1;
                    }

                    let token = opt_break!(self.parse_emphasis(c, n));
                    s.push_token(token);
                    s.update();
                }

                // just advance
                _ => s.advance()
            }
        }

        if self.cur.valid(s.pm_last) {
            s.push_chunk();
        }

        s.tokens
    }
}