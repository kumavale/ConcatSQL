extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::LitStr;
use nom::{
    IResult,
    branch::alt,
    character::complete::{char, none_of},
    multi::{many0, many1},
};

#[derive(Debug)]
enum Query {
    Lit(String),
    Param(String),
}

struct FormatParser {
    input: String,
}

impl FormatParser {
    /// EBNF
    /// format      = ( lit | param )*
    /// lit         = char+
    /// param       = brace_open char+ brace_close
    /// brace_open  = '{'
    /// brace_close = '}'
    /// char        = std::Char
    fn parse(&mut self) -> Result<TokenStream, syn::parse::Error> {
        let query = FormatParser::format(&self.input).unwrap().1;
        let mut lits = vec![];
        let mut params = vec![];
        for q in query.into_iter() {
            match q {
                Query::Lit(s) => {
                    lits.push(quote!{ Some( #s ) });
                }
                Query::Param(p) => {
                    lits.push(quote!{ None });
                    params.push(Ident::new(&p, Span::call_site()));
                }
            }
        }
        Ok(quote! {
            WrapString::_init(
                vec![ #(#lits),* ],
                vec![ #(#params.to_value()),* ],
            )
        }.into())
    }

    fn format(input: &str) -> IResult<&str, Vec<Query>> {
        many0(alt((FormatParser::param, FormatParser::lit)))(input)
    }

    fn lit(input: &str) -> IResult<&str, Query> {
        let (input, lit) = many1(none_of("{"))(input)?;
        Ok((input, Query::Lit(lit.into_iter().collect())))
    }

    fn param(input: &str) -> IResult<&str, Query> {
        let (input, _) = FormatParser::brace_open(input)?;
        let (input, param) = many1(none_of("}"))(input)?;
        let (input, _) = FormatParser::brace_close(input)?;
        Ok((input, Query::Param(param.into_iter().collect())))
    }

    fn brace_open(input: &str) -> IResult<&str, char> {
        char('{')(input)
    }

    fn brace_close(input: &str) -> IResult<&str, char> {
        char('}')(input)
    }
}

#[proc_macro]
pub fn query(item: TokenStream) -> TokenStream {
    let item_lit: LitStr = syn::parse2(item.into()).unwrap();
    let mut parser = FormatParser {
        input: item_lit.value(),
    };
    parser.parse().unwrap()
}
