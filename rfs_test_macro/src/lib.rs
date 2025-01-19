extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr, parse::Parser};
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn rfs_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block; // Extract the function body

    // Parse the attributes
    let attr_parser = |stream: TokenStream2| -> Result<(Option<String>, Option<String>), syn::Error> {
        let mut config = None;
        let mut start_point = None;

        // Manually parse the attributes
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("config") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                config = Some(lit.value());
            } else if meta.path.is_ident("start_point") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                start_point = Some(lit.value());
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
    let config = config.unwrap_or_else(|| {
        r#"---
        - !directory
            name: test
            content:
              - !file
                  name: test.txt
                  content:
                    !inline_text "Hello, world!"
        "#
        .to_string()
    });
    let start_point = start_point.unwrap_or_else(|| ".".to_string());

    // Generate the test function
    let expanded = quote! {
        #[test]
        fn #fn_name() {
            use rfs_tester::{FsTester, FileContent};
            use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

            // Use the provided parameters
            let config_str = #config;
            let start_point = #start_point;

            // Create the temporary file system
            let tester = FsTester::new(config_str, start_point);

            // Run the test
            tester.perform_fs_test(|dirname| {
                println!("Test directory: {}", dirname); // Debug output
                #fn_block
            });
        }
    };

    // Print the generated code for debugging
    println!("Generated test function:\n{}", expanded);

    TokenStream::from(expanded)
}