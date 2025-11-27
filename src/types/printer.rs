use super::types::Type;

/// Print a type as a string
pub fn print_type(ty: &Type) -> String {
    match ty {
        Type::Any => "any".to_string(),
        Type::Unknown => "unknown".to_string(),
        Type::Never => "never".to_string(),
        Type::Void => "void".to_string(),
        Type::Undefined => "undefined".to_string(),
        Type::Null => "null".to_string(),
        Type::String => "string".to_string(),
        Type::Number => "number".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Symbol => "symbol".to_string(),
        Type::BigInt => "bigint".to_string(),

        Type::StringLiteral(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Type::NumberLiteral(n) => format!("{}", n),
        Type::BooleanLiteral(b) => format!("{}", b),
        Type::BigIntLiteral(s) => format!("{}n", s),

        Type::Array(element) => format!("{}[]", print_type(element)),

        Type::Tuple(elements) => {
            let elems: Vec<String> = elements.iter().map(print_type).collect();
            format!("[{}]", elems.join(", "))
        }

        Type::Object(obj) => {
            if obj.properties.is_empty()
                && obj.index_signatures.is_empty()
                && obj.call_signatures.is_empty()
            {
                return "{}".to_string();
            }

            let mut parts = Vec::new();

            // Print properties
            for (name, prop) in &obj.properties {
                let readonly = if prop.readonly { "readonly " } else { "" };
                let optional = if prop.optional { "?" } else { "" };
                parts.push(format!(
                    "{}{}{}: {}",
                    readonly,
                    name,
                    optional,
                    print_type(&prop.ty)
                ));
            }

            // Print index signatures
            for sig in &obj.index_signatures {
                let readonly = if sig.readonly { "readonly " } else { "" };
                parts.push(format!(
                    "{}[key: {}]: {}",
                    readonly,
                    print_type(&sig.key_type),
                    print_type(&sig.value_type)
                ));
            }

            format!("{{ {} }}", parts.join("; "))
        }

        Type::Function(func) => {
            let type_params = if func.type_parameters.is_empty() {
                String::new()
            } else {
                let params: Vec<String> = func
                    .type_parameters
                    .iter()
                    .map(|p| {
                        let constraint = p
                            .constraint
                            .as_ref()
                            .map(|c| format!(" extends {}", print_type(c)))
                            .unwrap_or_default();
                        format!("{}{}", p.name, constraint)
                    })
                    .collect();
                format!("<{}>", params.join(", "))
            };

            let params: Vec<String> = func
                .parameters
                .iter()
                .map(|p| {
                    let rest = if p.rest { "..." } else { "" };
                    let optional = if p.optional { "?" } else { "" };
                    format!("{}{}{}: {}", rest, p.name, optional, print_type(&p.ty))
                })
                .collect();

            format!(
                "{}({}) => {}",
                type_params,
                params.join(", "),
                print_type(&func.return_type)
            )
        }

        Type::Union(members) => {
            let parts: Vec<String> = members.iter().map(print_type).collect();
            parts.join(" | ")
        }

        Type::Intersection(members) => {
            let parts: Vec<String> = members.iter().map(print_type).collect();
            parts.join(" & ")
        }

        Type::TypeParameter(param) => {
            let constraint = param
                .constraint
                .as_ref()
                .map(|c| format!(" extends {}", print_type(c)))
                .unwrap_or_default();
            format!("{}{}", param.name, constraint)
        }

        Type::Conditional(cond) => {
            format!(
                "{} extends {} ? {} : {}",
                print_type(&cond.check_type),
                print_type(&cond.extends_type),
                print_type(&cond.true_type),
                print_type(&cond.false_type)
            )
        }

        Type::Mapped(mapped) => {
            let readonly = match mapped.readonly_modifier {
                Some(true) => "+readonly ",
                Some(false) => "-readonly ",
                None => "",
            };
            let optional = match mapped.optional_modifier {
                Some(true) => "+?",
                Some(false) => "-?",
                None => "",
            };
            format!(
                "{{ {}[{} in {}]{}: {} }}",
                readonly,
                mapped.type_parameter,
                print_type(&mapped.constraint),
                optional,
                print_type(&mapped.template_type)
            )
        }

        Type::Index(ty) => format!("keyof {}", print_type(ty)),

        Type::IndexedAccess(access) => {
            format!(
                "{}[{}]",
                print_type(&access.object_type),
                print_type(&access.index_type)
            )
        }

        Type::Reference(ref_type) => {
            if ref_type.type_arguments.is_empty() {
                ref_type.name.clone()
            } else {
                let args: Vec<String> = ref_type.type_arguments.iter().map(print_type).collect();
                format!("{}<{}>", ref_type.name, args.join(", "))
            }
        }

        Type::This => "this".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::types::{
        ConditionalType, FunctionType, IndexSignature, IndexedAccessType, MappedType, ObjectType,
        Parameter, Property, TypeParameter, TypeReference,
    };

    #[test]
    fn test_print_primitives() {
        assert_eq!(print_type(&Type::Any), "any");
        assert_eq!(print_type(&Type::Unknown), "unknown");
        assert_eq!(print_type(&Type::Never), "never");
        assert_eq!(print_type(&Type::Void), "void");
        assert_eq!(print_type(&Type::Undefined), "undefined");
        assert_eq!(print_type(&Type::Null), "null");
        assert_eq!(print_type(&Type::String), "string");
        assert_eq!(print_type(&Type::Number), "number");
        assert_eq!(print_type(&Type::Boolean), "boolean");
        assert_eq!(print_type(&Type::Symbol), "symbol");
        assert_eq!(print_type(&Type::BigInt), "bigint");
    }

    #[test]
    fn test_print_string_literal() {
        assert_eq!(
            print_type(&Type::StringLiteral("hello".to_string())),
            "\"hello\""
        );
        assert_eq!(
            print_type(&Type::StringLiteral("with\"quote".to_string())),
            "\"with\\\"quote\""
        );
    }

    #[test]
    fn test_print_number_literal() {
        assert_eq!(print_type(&Type::NumberLiteral(42.0)), "42");
        assert_eq!(print_type(&Type::NumberLiteral(1.5)), "1.5");
    }

    #[test]
    fn test_print_boolean_literal() {
        assert_eq!(print_type(&Type::BooleanLiteral(true)), "true");
        assert_eq!(print_type(&Type::BooleanLiteral(false)), "false");
    }

    #[test]
    fn test_print_bigint_literal() {
        assert_eq!(print_type(&Type::BigIntLiteral("123".to_string())), "123n");
    }

    #[test]
    fn test_print_array() {
        assert_eq!(print_type(&Type::Array(Box::new(Type::String))), "string[]");
        assert_eq!(print_type(&Type::Array(Box::new(Type::Number))), "number[]");
    }

    #[test]
    fn test_print_tuple() {
        assert_eq!(
            print_type(&Type::Tuple(vec![Type::String, Type::Number])),
            "[string, number]"
        );
        assert_eq!(print_type(&Type::Tuple(vec![])), "[]");
    }

    #[test]
    fn test_print_union() {
        assert_eq!(
            print_type(&Type::Union(vec![Type::String, Type::Number])),
            "string | number"
        );
    }

    #[test]
    fn test_print_intersection() {
        let obj1 = Type::Object(ObjectType::default());
        let obj2 = Type::Object(ObjectType::default());
        assert_eq!(print_type(&Type::Intersection(vec![obj1, obj2])), "{} & {}");
    }

    #[test]
    fn test_print_empty_object() {
        assert_eq!(print_type(&Type::Object(ObjectType::default())), "{}");
    }

    #[test]
    fn test_print_object_with_properties() {
        let mut obj = ObjectType::default();
        obj.properties.insert(
            "name".to_string(),
            Property {
                name: "name".to_string(),
                ty: Box::new(Type::String),
                optional: false,
                readonly: false,
            },
        );

        let result = print_type(&Type::Object(obj));
        assert!(result.contains("name: string"));
    }

    #[test]
    fn test_print_object_with_optional_property() {
        let mut obj = ObjectType::default();
        obj.properties.insert(
            "age".to_string(),
            Property {
                name: "age".to_string(),
                ty: Box::new(Type::Number),
                optional: true,
                readonly: false,
            },
        );

        let result = print_type(&Type::Object(obj));
        assert!(result.contains("age?"));
    }

    #[test]
    fn test_print_object_with_readonly_property() {
        let mut obj = ObjectType::default();
        obj.properties.insert(
            "id".to_string(),
            Property {
                name: "id".to_string(),
                ty: Box::new(Type::Number),
                optional: false,
                readonly: true,
            },
        );

        let result = print_type(&Type::Object(obj));
        assert!(result.contains("readonly"));
    }

    #[test]
    fn test_print_object_with_index_signature() {
        let mut obj = ObjectType::default();
        obj.index_signatures.push(IndexSignature {
            key_type: Box::new(Type::String),
            value_type: Box::new(Type::Number),
            readonly: false,
        });

        let result = print_type(&Type::Object(obj));
        assert!(result.contains("[key: string]: number"));
    }

    #[test]
    fn test_print_function_simple() {
        let func = FunctionType {
            type_parameters: vec![],
            parameters: vec![Parameter {
                name: "x".to_string(),
                ty: Box::new(Type::Number),
                optional: false,
                rest: false,
            }],
            return_type: Box::new(Type::String),
            this_type: None,
        };

        assert_eq!(print_type(&Type::Function(func)), "(x: number) => string");
    }

    #[test]
    fn test_print_function_with_type_params() {
        let func = FunctionType {
            type_parameters: vec![TypeParameter {
                name: "T".to_string(),
                constraint: None,
                default: None,
            }],
            parameters: vec![Parameter {
                name: "value".to_string(),
                ty: Box::new(Type::TypeParameter(TypeParameter {
                    name: "T".to_string(),
                    constraint: None,
                    default: None,
                })),
                optional: false,
                rest: false,
            }],
            return_type: Box::new(Type::TypeParameter(TypeParameter {
                name: "T".to_string(),
                constraint: None,
                default: None,
            })),
            this_type: None,
        };

        let result = print_type(&Type::Function(func));
        assert!(result.starts_with("<T>"));
        assert!(result.contains("(value: T) => T"));
    }

    #[test]
    fn test_print_function_optional_param() {
        let func = FunctionType {
            type_parameters: vec![],
            parameters: vec![Parameter {
                name: "x".to_string(),
                ty: Box::new(Type::Number),
                optional: true,
                rest: false,
            }],
            return_type: Box::new(Type::Void),
            this_type: None,
        };

        assert_eq!(print_type(&Type::Function(func)), "(x?: number) => void");
    }

    #[test]
    fn test_print_function_rest_param() {
        let func = FunctionType {
            type_parameters: vec![],
            parameters: vec![Parameter {
                name: "args".to_string(),
                ty: Box::new(Type::Array(Box::new(Type::String))),
                optional: false,
                rest: true,
            }],
            return_type: Box::new(Type::Void),
            this_type: None,
        };

        assert_eq!(
            print_type(&Type::Function(func)),
            "(...args: string[]) => void"
        );
    }

    #[test]
    fn test_print_type_parameter() {
        let param = Type::TypeParameter(TypeParameter {
            name: "T".to_string(),
            constraint: None,
            default: None,
        });
        assert_eq!(print_type(&param), "T");
    }

    #[test]
    fn test_print_type_parameter_with_constraint() {
        let param = Type::TypeParameter(TypeParameter {
            name: "T".to_string(),
            constraint: Some(Box::new(Type::String)),
            default: None,
        });
        assert_eq!(print_type(&param), "T extends string");
    }

    #[test]
    fn test_print_conditional() {
        let cond = Type::Conditional(ConditionalType {
            check_type: Box::new(Type::TypeParameter(TypeParameter {
                name: "T".to_string(),
                constraint: None,
                default: None,
            })),
            extends_type: Box::new(Type::String),
            true_type: Box::new(Type::Number),
            false_type: Box::new(Type::Boolean),
        });

        assert_eq!(print_type(&cond), "T extends string ? number : boolean");
    }

    #[test]
    fn test_print_mapped() {
        let mapped = Type::Mapped(MappedType {
            type_parameter: "K".to_string(),
            constraint: Box::new(Type::String),
            template_type: Box::new(Type::Number),
            readonly_modifier: None,
            optional_modifier: None,
        });

        assert_eq!(print_type(&mapped), "{ [K in string]: number }");
    }

    #[test]
    fn test_print_mapped_with_modifiers() {
        let mapped = Type::Mapped(MappedType {
            type_parameter: "K".to_string(),
            constraint: Box::new(Type::String),
            template_type: Box::new(Type::Number),
            readonly_modifier: Some(true),
            optional_modifier: Some(true),
        });

        let result = print_type(&mapped);
        assert!(result.contains("+readonly"));
        assert!(result.contains("+?"));
    }

    #[test]
    fn test_print_index() {
        let index = Type::Index(Box::new(Type::Object(ObjectType::default())));
        assert_eq!(print_type(&index), "keyof {}");
    }

    #[test]
    fn test_print_indexed_access() {
        let access = Type::IndexedAccess(IndexedAccessType {
            object_type: Box::new(Type::TypeParameter(TypeParameter {
                name: "T".to_string(),
                constraint: None,
                default: None,
            })),
            index_type: Box::new(Type::StringLiteral("key".to_string())),
        });

        assert_eq!(print_type(&access), "T[\"key\"]");
    }

    #[test]
    fn test_print_reference_simple() {
        let reference = Type::Reference(TypeReference {
            name: "Array".to_string(),
            type_arguments: vec![],
        });
        assert_eq!(print_type(&reference), "Array");
    }

    #[test]
    fn test_print_reference_with_args() {
        let reference = Type::Reference(TypeReference {
            name: "Map".to_string(),
            type_arguments: vec![Type::String, Type::Number],
        });
        assert_eq!(print_type(&reference), "Map<string, number>");
    }

    #[test]
    fn test_print_this() {
        assert_eq!(print_type(&Type::This), "this");
    }
}
