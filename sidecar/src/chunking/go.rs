use crate::chunking::languages::TSLanguageConfig;

pub fn go_language_config() -> TSLanguageConfig {
    TSLanguageConfig {
        language_ids: &["Go"],
        file_extensions: &["go"],
        grammar: tree_sitter_go::language,
        namespaces: vec![vec![
            "const",
            "func",
            "var",
            "struct",
            "interface",
            "type",
            "package",
            "label",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()],
        documentation_query: vec!["((comment) @comment
            (#match? @comment \"^//\")) @docComment"
            .to_owned()],
        function_query: vec!["[(function_declaration
            name: (identifier) @identifier
            parameters: (parameter_list)? @parameters
            result: (
              (type_identifier) @return_type
            )?
            body: (block) @body
          ) (method_declaration 
            name: (field_identifier) @identifier
            parameters: (parameter_list) @parameters
            result: (type_identifier) @result_type)
           (method_declaration
            receiver: (parameter_list
              (parameter_declaration
                name: (identifier) @receiver_name
                type: (type_identifier) @receiver_type
              )
            )
            name: (field_identifier) @method_name
            parameters: (parameter_list)? @parameters
            result: (
                (pointer_type
                  (type_identifier) @return_type
              )
            )?
          )
          (method_declaration
            receiver: (parameter_list
              (parameter_declaration
                name: (identifier) @receiver_name
                type: ((pointer_type (type_identifier) @type_identifier))
              )
            )
            name: (field_identifier) @method_name
            parameters: (parameter_list)? @parameters
            result: (
                (pointer_type
                  (type_identifier) @return_type
              )
            )?
          )] @function"
            .to_owned()],
        construct_types: vec![
            "source_file",
            "type_declaration",
            "type_spec",
            "struct_type",
            "interface_type",
            "function_declaration",
            "method_declaration",
            "package_clause",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        expression_statements: vec![
            "short_var_declaration",
            "assignment_statement",
            "call_expression",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect(),
        class_query: vec!["[
                (type_declaration (type_spec name: (type_identifier)? @identifier))
                (type_declaration (struct_type name: (type_identifier)? @identifier))
                (type_declaration (interface_type name: (type_identifier)? @identifier))
            ] @class_declaration"
            .to_owned()],
        r#type_query: vec![],
        namespace_types: vec![],
        hoverable_query: r#"
        [(identifier)
         (field_identifier)
         (type_identifier)] @hoverable
        "#
        .to_owned(),
        comment_prefix: "//".to_owned(),
        end_of_line: None,
        import_statement: vec!["[(import_declaration)] @import_type".to_owned()],
        block_start: Some("{".to_owned()),
        variable_identifier_queries: vec![
            "(short_var_declaration left: (expression_list (identifier) @identifier))".to_owned(),
        ],
        outline_query: Some(
            r#"
            (type_declaration
                (type_spec
                    name: (type_identifier) @definition.class.name
                )
            ) @definition.class
            
            (method_declaration
                name: (field_identifier) @function.name
                body: (block) @function.body
            ) @definition.method
            
            (function_declaration
                name: (identifier) @function.name
                body: (block) @function.body
            ) @definition.function
            "#
            .to_owned(),
        ),
        excluded_file_paths: vec![],
        language_str: "go".to_owned(),
    }
}
