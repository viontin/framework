//! Parses the #[domain(name = "...", allows = [...])] attribute syntax.

use syn::parse::{Parse, ParseStream};
use syn::{Lit, Token};

/// Parsed `#[domain(...)]` attribute.
pub struct DomainAttr {
    pub name: String,
    pub allows: Vec<String>,
    pub provides: Vec<String>,
}

impl Parse for DomainAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = String::new();
        let mut allows: Vec<String> = Vec::new();
        let mut provides: Vec<String> = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let _: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "name" => {
                    let lit: Lit = input.parse()?;
                    name = match lit {
                        Lit::Str(s) => s.value(),
                        _ => return Err(syn::Error::new(lit.span(), "expected string literal")),
                    };
                }
                "allows" => {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        allows.push(ident.to_string());
                        let _ = content.parse::<Token![,]>();
                    }
                }
                "provides" => {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        provides.push(ident.to_string());
                        let _ = content.parse::<Token![,]>();
                    }
                }
                other => {
                    return Err(syn::Error::new(key.span(), format!("unknown key: {}", other)));
                }
            }

            let _ = input.parse::<Token![,]>();
        }

        if name.is_empty() {
            return Err(syn::Error::new(input.span(), "expected domain name: #[domain(name = \"...\")]"));
        }

        Ok(DomainAttr { name, allows, provides })
    }
}
