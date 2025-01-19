extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr, Meta, parse::Parser};
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn rfs_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Парсим входную функцию
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    // Парсим атрибуты
    let attr_parser = |stream: TokenStream2| -> Result<(Option<String>, Option<String>), syn::Error> {
        let mut config = None;
        let mut start_point = None;

        // Парсим атрибуты вручную
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

    // Парсим атрибуты
    let (config, start_point) = match attr_parser(attr.into()) {
        Ok(result) => result,
        Err(err) => return err.to_compile_error().into(),
    };

    // Значения по умолчанию
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

    // Генерируем код для обертки теста
    let expanded = quote! {
        #[test]
        fn #fn_name() {
            use rfs_tester::{FsTester, FileContent};
            use rfs_tester::config::{Configuration, ConfigEntry, DirectoryConf, FileConf};

            // Используем переданные параметры
            let config_str = #config;
            let start_point = #start_point;

            // Создаем временную файловую систему
            let tester = FsTester::new(config_str, start_point);

            // Выполняем тест
            tester.perform_fs_test(|dirname| {
                #input_fn
                Ok(())
            });
        }
    };

    TokenStream::from(expanded)
}