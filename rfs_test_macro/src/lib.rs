extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Expr, parse::Parser};
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn rfs_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;

    // Parse the attributes
    let attr_parser = |stream: TokenStream2| -> Result<(Option<Expr>, Option<Expr>), syn::Error> {
        let mut config = None;
        let mut start_point = None;

        // Manually parse the attributes
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("config") {
                let value = meta.value()?;
                config = Some(value.parse()?);
            } else if meta.path.is_ident("start_point") {
                let value = meta.value()?;
                start_point = Some(value.parse()?);
            } else {
                return Err(meta.error("unsupported attribute"));
            }
            Ok(())
        });

        parser.parse2(stream)?;
        Ok((config, start_point))
    };

    // Parse the attributes
    let (config, start_point) = match attr_parser(attr.into()) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };

    // Default values
    let config = config.unwrap_or_else(|| syn::parse_str(r#"---
        - !directory
            name: test
            content:
              - !file
                  name: test.txt
                  content:
                    !inline_text "Hello, world!"
        "#).unwrap());
    let start_point = start_point.unwrap_or_else(|| syn::parse_str(r#"".""#).unwrap());

    // Generate the test function
    let expanded = quote! {
        #[test]
        fn #fn_name() {
            use rfs_tester::{FsTester, FileContent};
            use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

            // Use the provided parameters
            let config_str: &str = #config;
            let start_point: &str = #start_point;

            // Create the temporary file system
            let tester = FsTester::new(config_str, start_point);

            // Run the test
            tester.perform_fs_test(|dirname| {
                #fn_block
            });
        }
    };

    TokenStream::from(expanded)
}