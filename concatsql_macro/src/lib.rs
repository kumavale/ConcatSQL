extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::LitStr;
use nom::{
    IResult,
    branch::alt,
    bytes::complete::tag,
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
    /// format      = ( brace_open | brace_close | param | lit )*
    /// lit         = char+
    /// param       = '{' char+ '}'
    /// brace_open  = '{{'
    /// brace_close = '}}'
    /// char        = std::Char
    fn parse(&mut self) -> Result<TokenStream, nom::Err<nom::error::Error<&str>>> {
        let (_, query) = FormatParser::format(&self.input)?;
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
        many0(alt((FormatParser::brace_open, FormatParser::brace_close, FormatParser::param, FormatParser::lit)))(input)
    }

    fn lit(input: &str) -> IResult<&str, Query> {
        let (input, lit) = many1(none_of("{}"))(input)?;
        Ok((input, Query::Lit(lit.into_iter().collect())))
    }

    fn param(input: &str) -> IResult<&str, Query> {
        let (input, _) = char('{')(input)?;
        let (input, param) = many1(none_of("}"))(input)?;
        let (input, _) = char('}')(input)?;
        Ok((input, Query::Param(param.into_iter().collect())))
    }

    fn brace_open(input: &str) -> IResult<&str, Query> {
        let (input, _) = tag("{{")(input)?;
        Ok((input, Query::Lit("{".to_string())))
    }

    fn brace_close(input: &str) -> IResult<&str, Query> {
        let (input, _) = tag("}}")(input)?;
        Ok((input, Query::Lit("}".to_string())))
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
