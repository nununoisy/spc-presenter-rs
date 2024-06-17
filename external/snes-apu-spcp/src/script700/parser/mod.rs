pub mod script_area;
pub mod data_area;

macro_rules! expect_token {
    ($token_iter: expr, $token: ident) => {
        match $token_iter.next() {
            Some((crate::script700::lexer::Token::$token(s), context)) => (*s, context.clone()),
            Some((other, context)) => {
                println!(concat!("[Script700] {}: parse error: expected ", stringify!($token), ", got {:?}"), context, other);
                return None;
            },
            None => return None
        }
    };
}

pub(super) use expect_token;


