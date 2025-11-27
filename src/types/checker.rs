use std::collections::HashMap;

use super::types::{Type, TypeId};

/// The type checker - performs type inference and checking
pub struct TypeChecker {
    /// Type cache - maps type ids to types
    types: HashMap<TypeId, Type>,
    /// Next type id
    next_id: u32,
    /// Built-in types
    builtin_types: BuiltinTypes,
}

/// Pre-defined builtin types
struct BuiltinTypes {
    any: TypeId,
    unknown: TypeId,
    never: TypeId,
    void: TypeId,
    undefined: TypeId,
    null: TypeId,
    string: TypeId,
    number: TypeId,
    boolean: TypeId,
    symbol: TypeId,
    bigint: TypeId,
    object: TypeId,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            types: HashMap::new(),
            next_id: 0,
            builtin_types: BuiltinTypes {
                any: TypeId::new(0),
                unknown: TypeId::new(1),
                never: TypeId::new(2),
                void: TypeId::new(3),
                undefined: TypeId::new(4),
                null: TypeId::new(5),
                string: TypeId::new(6),
                number: TypeId::new(7),
                boolean: TypeId::new(8),
                symbol: TypeId::new(9),
                bigint: TypeId::new(10),
                object: TypeId::new(11),
            },
        };

        // Register builtin types
        checker.register_builtin_types();

        checker
    }

    fn register_builtin_types(&mut self) {
        self.types.insert(self.builtin_types.any, Type::Any);
        self.types.insert(self.builtin_types.unknown, Type::Unknown);
        self.types.insert(self.builtin_types.never, Type::Never);
        self.types.insert(self.builtin_types.void, Type::Void);
        self.types
            .insert(self.builtin_types.undefined, Type::Undefined);
        self.types.insert(self.builtin_types.null, Type::Null);
        self.types.insert(self.builtin_types.string, Type::String);
        self.types.insert(self.builtin_types.number, Type::Number);
        self.types.insert(self.builtin_types.boolean, Type::Boolean);
        self.types.insert(self.builtin_types.symbol, Type::Symbol);
        self.types.insert(self.builtin_types.bigint, Type::BigInt);
        self.types.insert(
            self.builtin_types.object,
            Type::Object(super::types::ObjectType::default()),
        );

        self.next_id = 12;
    }

    /// Get the any type
    pub fn any_type(&self) -> TypeId {
        self.builtin_types.any
    }

    /// Get the unknown type
    pub fn unknown_type(&self) -> TypeId {
        self.builtin_types.unknown
    }

    /// Get the never type
    pub fn never_type(&self) -> TypeId {
        self.builtin_types.never
    }

    /// Get the void type
    pub fn void_type(&self) -> TypeId {
        self.builtin_types.void
    }

    /// Get the undefined type
    pub fn undefined_type(&self) -> TypeId {
        self.builtin_types.undefined
    }

    /// Get the null type
    pub fn null_type(&self) -> TypeId {
        self.builtin_types.null
    }

    /// Get the string type
    pub fn string_type(&self) -> TypeId {
        self.builtin_types.string
    }

    /// Get the number type
    pub fn number_type(&self) -> TypeId {
        self.builtin_types.number
    }

    /// Get the boolean type
    pub fn boolean_type(&self) -> TypeId {
        self.builtin_types.boolean
    }

    /// Get the symbol type
    pub fn symbol_type(&self) -> TypeId {
        self.builtin_types.symbol
    }

    /// Get the bigint type
    pub fn bigint_type(&self) -> TypeId {
        self.builtin_types.bigint
    }

    /// Get a type by id
    pub fn get_type(&self, id: TypeId) -> Option<&Type> {
        self.types.get(&id)
    }

    /// Create a new type and return its id
    pub fn create_type(&mut self, ty: Type) -> TypeId {
        let id = TypeId::new(self.next_id);
        self.next_id += 1;
        self.types.insert(id, ty);
        id
    }

    /// Create a string literal type
    pub fn string_literal_type(&mut self, value: String) -> TypeId {
        self.create_type(Type::StringLiteral(value))
    }

    /// Create a number literal type
    pub fn number_literal_type(&mut self, value: f64) -> TypeId {
        self.create_type(Type::NumberLiteral(value))
    }

    /// Create a boolean literal type
    pub fn boolean_literal_type(&mut self, value: bool) -> TypeId {
        self.create_type(Type::BooleanLiteral(value))
    }

    /// Create an array type
    pub fn array_type(&mut self, element_type: TypeId) -> TypeId {
        let element = self.get_type(element_type).cloned().unwrap_or(Type::Any);
        self.create_type(Type::Array(Box::new(element)))
    }

    /// Create a union type
    pub fn union_type(&mut self, types: Vec<TypeId>) -> TypeId {
        let members: Vec<Type> = types
            .iter()
            .filter_map(|id| self.get_type(*id).cloned())
            .collect();

        // Simplify union if possible
        if members.len() == 1 {
            return types[0];
        }

        self.create_type(Type::Union(members))
    }

    /// Create an intersection type
    pub fn intersection_type(&mut self, types: Vec<TypeId>) -> TypeId {
        let members: Vec<Type> = types
            .iter()
            .filter_map(|id| self.get_type(*id).cloned())
            .collect();

        if members.len() == 1 {
            return types[0];
        }

        self.create_type(Type::Intersection(members))
    }

    /// Check if one type is assignable to another
    pub fn is_assignable_to(&self, source: TypeId, target: TypeId) -> bool {
        if source == target {
            return true;
        }

        let source_type = match self.get_type(source) {
            Some(t) => t,
            None => return false,
        };

        let target_type = match self.get_type(target) {
            Some(t) => t,
            None => return false,
        };

        source_type.is_assignable_to(target_type)
    }

    /// Get the type of a literal value from source text
    pub fn type_of_literal(&mut self, kind: &str, text: &str) -> TypeId {
        match kind {
            "string" | "template_string" => {
                // Remove quotes
                let value = text
                    .trim_start_matches(['"', '\'', '`'])
                    .trim_end_matches(['"', '\'', '`']);
                self.string_literal_type(value.to_string())
            }
            "number" => {
                if let Ok(n) = text.parse::<f64>() {
                    self.number_literal_type(n)
                } else {
                    self.number_type()
                }
            }
            "true" => self.boolean_literal_type(true),
            "false" => self.boolean_literal_type(false),
            "null" => self.null_type(),
            "undefined" => self.undefined_type(),
            _ => self.any_type(),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_checker_new() {
        let checker = TypeChecker::new();
        // Should have builtin types registered
        assert!(checker.get_type(checker.any_type()).is_some());
        assert!(checker.get_type(checker.string_type()).is_some());
        assert!(checker.get_type(checker.number_type()).is_some());
    }

    #[test]
    fn test_type_checker_default() {
        let checker = TypeChecker::default();
        assert!(checker.get_type(checker.any_type()).is_some());
    }

    #[test]
    fn test_builtin_types() {
        let checker = TypeChecker::new();

        let any = checker.get_type(checker.any_type()).unwrap();
        assert!(matches!(any, Type::Any));

        let unknown = checker.get_type(checker.unknown_type()).unwrap();
        assert!(matches!(unknown, Type::Unknown));

        let never = checker.get_type(checker.never_type()).unwrap();
        assert!(matches!(never, Type::Never));

        let void = checker.get_type(checker.void_type()).unwrap();
        assert!(matches!(void, Type::Void));

        let undefined = checker.get_type(checker.undefined_type()).unwrap();
        assert!(matches!(undefined, Type::Undefined));

        let null = checker.get_type(checker.null_type()).unwrap();
        assert!(matches!(null, Type::Null));

        let string = checker.get_type(checker.string_type()).unwrap();
        assert!(matches!(string, Type::String));

        let number = checker.get_type(checker.number_type()).unwrap();
        assert!(matches!(number, Type::Number));

        let boolean = checker.get_type(checker.boolean_type()).unwrap();
        assert!(matches!(boolean, Type::Boolean));

        let symbol = checker.get_type(checker.symbol_type()).unwrap();
        assert!(matches!(symbol, Type::Symbol));

        let bigint = checker.get_type(checker.bigint_type()).unwrap();
        assert!(matches!(bigint, Type::BigInt));
    }

    #[test]
    fn test_create_type() {
        let mut checker = TypeChecker::new();

        let id = checker.create_type(Type::String);
        let ty = checker.get_type(id).unwrap();
        assert!(matches!(ty, Type::String));
    }

    #[test]
    fn test_string_literal_type() {
        let mut checker = TypeChecker::new();

        let id = checker.string_literal_type("hello".to_string());
        let ty = checker.get_type(id).unwrap();

        if let Type::StringLiteral(s) = ty {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_number_literal_type() {
        let mut checker = TypeChecker::new();

        let id = checker.number_literal_type(42.0);
        let ty = checker.get_type(id).unwrap();

        if let Type::NumberLiteral(n) = ty {
            assert!((n - 42.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected NumberLiteral");
        }
    }

    #[test]
    fn test_boolean_literal_type() {
        let mut checker = TypeChecker::new();

        let true_id = checker.boolean_literal_type(true);
        let false_id = checker.boolean_literal_type(false);

        let true_ty = checker.get_type(true_id).unwrap();
        let false_ty = checker.get_type(false_id).unwrap();

        assert!(matches!(true_ty, Type::BooleanLiteral(true)));
        assert!(matches!(false_ty, Type::BooleanLiteral(false)));
    }

    #[test]
    fn test_array_type() {
        let mut checker = TypeChecker::new();

        let string_type = checker.string_type();
        let array_id = checker.array_type(string_type);

        let ty = checker.get_type(array_id).unwrap();
        assert!(matches!(ty, Type::Array(_)));
    }

    #[test]
    fn test_union_type() {
        let mut checker = TypeChecker::new();

        let string_type = checker.string_type();
        let number_type = checker.number_type();

        let union_id = checker.union_type(vec![string_type, number_type]);
        let ty = checker.get_type(union_id).unwrap();

        if let Type::Union(members) = ty {
            assert_eq!(members.len(), 2);
        } else {
            panic!("Expected Union");
        }
    }

    #[test]
    fn test_union_type_single_member() {
        let mut checker = TypeChecker::new();

        let string_type = checker.string_type();
        let union_id = checker.union_type(vec![string_type]);

        // Single member union should return the member directly
        assert_eq!(union_id, string_type);
    }

    #[test]
    fn test_intersection_type() {
        let mut checker = TypeChecker::new();

        let id1 = checker.create_type(Type::Object(super::super::types::ObjectType::default()));
        let id2 = checker.create_type(Type::Object(super::super::types::ObjectType::default()));

        let intersection_id = checker.intersection_type(vec![id1, id2]);
        let ty = checker.get_type(intersection_id).unwrap();

        assert!(matches!(ty, Type::Intersection(_)));
    }

    #[test]
    fn test_intersection_type_single_member() {
        let mut checker = TypeChecker::new();

        let string_type = checker.string_type();
        let intersection_id = checker.intersection_type(vec![string_type]);

        // Single member intersection should return the member directly
        assert_eq!(intersection_id, string_type);
    }

    #[test]
    fn test_is_assignable_same_type() {
        let checker = TypeChecker::new();

        let string_type = checker.string_type();
        assert!(checker.is_assignable_to(string_type, string_type));
    }

    #[test]
    fn test_is_assignable_to_any() {
        let checker = TypeChecker::new();

        let string_type = checker.string_type();
        let any_type = checker.any_type();

        assert!(checker.is_assignable_to(string_type, any_type));
    }

    #[test]
    fn test_is_assignable_from_any() {
        let checker = TypeChecker::new();

        let any_type = checker.any_type();
        let string_type = checker.string_type();

        assert!(checker.is_assignable_to(any_type, string_type));
    }

    #[test]
    fn test_is_assignable_to_unknown() {
        let checker = TypeChecker::new();

        let string_type = checker.string_type();
        let unknown_type = checker.unknown_type();

        assert!(checker.is_assignable_to(string_type, unknown_type));
    }

    #[test]
    fn test_never_assignable_to_everything() {
        let checker = TypeChecker::new();

        let never_type = checker.never_type();
        let string_type = checker.string_type();

        assert!(checker.is_assignable_to(never_type, string_type));
    }

    #[test]
    fn test_type_of_literal_string() {
        let mut checker = TypeChecker::new();

        let id = checker.type_of_literal("string", "\"hello\"");
        let ty = checker.get_type(id).unwrap();

        if let Type::StringLiteral(s) = ty {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected StringLiteral");
        }
    }

    #[test]
    fn test_type_of_literal_number() {
        let mut checker = TypeChecker::new();

        let id = checker.type_of_literal("number", "42");
        let ty = checker.get_type(id).unwrap();

        assert!(matches!(ty, Type::NumberLiteral(_)));
    }

    #[test]
    fn test_type_of_literal_boolean() {
        let mut checker = TypeChecker::new();

        let true_id = checker.type_of_literal("true", "true");
        let false_id = checker.type_of_literal("false", "false");

        assert!(matches!(
            checker.get_type(true_id).unwrap(),
            Type::BooleanLiteral(true)
        ));
        assert!(matches!(
            checker.get_type(false_id).unwrap(),
            Type::BooleanLiteral(false)
        ));
    }

    #[test]
    fn test_type_of_literal_null() {
        let mut checker = TypeChecker::new();

        let id = checker.type_of_literal("null", "null");
        assert_eq!(id, checker.null_type());
    }

    #[test]
    fn test_type_of_literal_undefined() {
        let mut checker = TypeChecker::new();

        let id = checker.type_of_literal("undefined", "undefined");
        assert_eq!(id, checker.undefined_type());
    }

    #[test]
    fn test_type_of_literal_unknown() {
        let mut checker = TypeChecker::new();

        let id = checker.type_of_literal("unknown_kind", "whatever");
        assert_eq!(id, checker.any_type());
    }

    #[test]
    fn test_get_nonexistent_type() {
        let checker = TypeChecker::new();

        let fake_id = TypeId::new(9999);
        assert!(checker.get_type(fake_id).is_none());
    }

    #[test]
    fn test_is_assignable_invalid_ids() {
        let checker = TypeChecker::new();

        let fake_id = TypeId::new(9999);
        let string_type = checker.string_type();

        assert!(!checker.is_assignable_to(fake_id, string_type));
        assert!(!checker.is_assignable_to(string_type, fake_id));
    }
}
