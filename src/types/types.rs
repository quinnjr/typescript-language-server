use std::collections::HashMap;

/// Unique identifier for a type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

impl TypeId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

bitflags::bitflags! {
    /// Flags describing type characteristics
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeFlags: u32 {
        const NONE = 0;

        // Primitive types
        const ANY = 1 << 0;
        const UNKNOWN = 1 << 1;
        const STRING = 1 << 2;
        const NUMBER = 1 << 3;
        const BOOLEAN = 1 << 4;
        const VOID = 1 << 5;
        const UNDEFINED = 1 << 6;
        const NULL = 1 << 7;
        const NEVER = 1 << 8;
        const SYMBOL = 1 << 9;
        const BIGINT = 1 << 10;

        // Literal types
        const STRING_LITERAL = 1 << 11;
        const NUMBER_LITERAL = 1 << 12;
        const BOOLEAN_LITERAL = 1 << 13;
        const BIGINT_LITERAL = 1 << 14;
        const ENUM_LITERAL = 1 << 15;

        // Compound types
        const OBJECT = 1 << 16;
        const UNION = 1 << 17;
        const INTERSECTION = 1 << 18;
        const FUNCTION = 1 << 19;
        const TUPLE = 1 << 20;
        const ARRAY = 1 << 21;

        // Special types
        const TYPE_PARAMETER = 1 << 22;
        const CONDITIONAL = 1 << 23;
        const MAPPED = 1 << 24;
        const INDEX = 1 << 25;
        const INDEXED_ACCESS = 1 << 26;

        // Combined flags
        const LITERAL = Self::STRING_LITERAL.bits() | Self::NUMBER_LITERAL.bits()
            | Self::BOOLEAN_LITERAL.bits() | Self::BIGINT_LITERAL.bits()
            | Self::ENUM_LITERAL.bits();

        const PRIMITIVE = Self::STRING.bits() | Self::NUMBER.bits() | Self::BOOLEAN.bits()
            | Self::VOID.bits() | Self::UNDEFINED.bits() | Self::NULL.bits()
            | Self::SYMBOL.bits() | Self::BIGINT.bits();

        const DEFINITE_FALSY = Self::VOID.bits() | Self::UNDEFINED.bits() | Self::NULL.bits();
    }
}

/// Represents a TypeScript type
#[derive(Debug, Clone)]
pub enum Type {
    /// Any type
    Any,
    /// Unknown type
    Unknown,
    /// Never type
    Never,
    /// Void type
    Void,
    /// Undefined type
    Undefined,
    /// Null type
    Null,
    /// String type
    String,
    /// Number type
    Number,
    /// Boolean type
    Boolean,
    /// Symbol type
    Symbol,
    /// BigInt type
    BigInt,
    /// String literal type
    StringLiteral(String),
    /// Number literal type
    NumberLiteral(f64),
    /// Boolean literal type
    BooleanLiteral(bool),
    /// BigInt literal type
    BigIntLiteral(String),
    /// Object type
    Object(ObjectType),
    /// Array type
    Array(Box<Type>),
    /// Tuple type
    Tuple(Vec<Type>),
    /// Function type
    Function(FunctionType),
    /// Union type (A | B)
    Union(Vec<Type>),
    /// Intersection type (A & B)
    Intersection(Vec<Type>),
    /// Type parameter (generic)
    TypeParameter(TypeParameter),
    /// Conditional type (T extends U ? X : Y)
    Conditional(ConditionalType),
    /// Mapped type ({ [K in keyof T]: ... })
    Mapped(MappedType),
    /// Index type (keyof T)
    Index(Box<Type>),
    /// Indexed access type (T[K])
    IndexedAccess(IndexedAccessType),
    /// Type reference (refers to a named type)
    Reference(TypeReference),
    /// This type
    This,
}

impl Type {
    /// Get the flags for this type
    pub fn flags(&self) -> TypeFlags {
        match self {
            Type::Any => TypeFlags::ANY,
            Type::Unknown => TypeFlags::UNKNOWN,
            Type::Never => TypeFlags::NEVER,
            Type::Void => TypeFlags::VOID,
            Type::Undefined => TypeFlags::UNDEFINED,
            Type::Null => TypeFlags::NULL,
            Type::String => TypeFlags::STRING,
            Type::Number => TypeFlags::NUMBER,
            Type::Boolean => TypeFlags::BOOLEAN,
            Type::Symbol => TypeFlags::SYMBOL,
            Type::BigInt => TypeFlags::BIGINT,
            Type::StringLiteral(_) => TypeFlags::STRING_LITERAL,
            Type::NumberLiteral(_) => TypeFlags::NUMBER_LITERAL,
            Type::BooleanLiteral(_) => TypeFlags::BOOLEAN_LITERAL,
            Type::BigIntLiteral(_) => TypeFlags::BIGINT_LITERAL,
            Type::Object(_) => TypeFlags::OBJECT,
            Type::Array(_) => TypeFlags::ARRAY,
            Type::Tuple(_) => TypeFlags::TUPLE,
            Type::Function(_) => TypeFlags::FUNCTION,
            Type::Union(_) => TypeFlags::UNION,
            Type::Intersection(_) => TypeFlags::INTERSECTION,
            Type::TypeParameter(_) => TypeFlags::TYPE_PARAMETER,
            Type::Conditional(_) => TypeFlags::CONDITIONAL,
            Type::Mapped(_) => TypeFlags::MAPPED,
            Type::Index(_) => TypeFlags::INDEX,
            Type::IndexedAccess(_) => TypeFlags::INDEXED_ACCESS,
            Type::Reference(_) => TypeFlags::OBJECT,
            Type::This => TypeFlags::OBJECT,
        }
    }

    /// Check if this type is assignable to another type
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        // Any is assignable to anything
        if matches!(self, Type::Any) {
            return true;
        }

        // Anything is assignable to any
        if matches!(target, Type::Any) {
            return true;
        }

        // Unknown accepts everything
        if matches!(target, Type::Unknown) {
            return true;
        }

        // Never is assignable to nothing (except itself)
        if matches!(self, Type::Never) {
            return true;
        }

        // Nothing is assignable to never
        if matches!(target, Type::Never) {
            return false;
        }

        // Same types are assignable
        match (self, target) {
            (Type::String, Type::String) => true,
            (Type::Number, Type::Number) => true,
            (Type::Boolean, Type::Boolean) => true,
            (Type::Void, Type::Void) => true,
            (Type::Undefined, Type::Undefined) => true,
            (Type::Undefined, Type::Void) => true,
            (Type::Null, Type::Null) => true,
            (Type::Symbol, Type::Symbol) => true,
            (Type::BigInt, Type::BigInt) => true,

            // Literals are assignable to their base types
            (Type::StringLiteral(_), Type::String) => true,
            (Type::NumberLiteral(_), Type::Number) => true,
            (Type::BooleanLiteral(_), Type::Boolean) => true,
            (Type::BigIntLiteral(_), Type::BigInt) => true,

            // Same literals
            (Type::StringLiteral(a), Type::StringLiteral(b)) => a == b,
            (Type::NumberLiteral(a), Type::NumberLiteral(b)) => (a - b).abs() < f64::EPSILON,
            (Type::BooleanLiteral(a), Type::BooleanLiteral(b)) => a == b,

            // Arrays
            (Type::Array(a), Type::Array(b)) => a.is_assignable_to(b),

            // Unions - source must be assignable to at least one member
            (_, Type::Union(members)) => members.iter().any(|m| self.is_assignable_to(m)),

            // Source union - all members must be assignable to target
            (Type::Union(members), _) => members.iter().all(|m| m.is_assignable_to(target)),

            // Intersections - source must be assignable to all members
            (_, Type::Intersection(members)) => members.iter().all(|m| self.is_assignable_to(m)),

            // TODO: More complex type relationships
            _ => false,
        }
    }
}

/// Object type (interface, class, etc.)
#[derive(Debug, Clone, Default)]
pub struct ObjectType {
    /// Properties of the object
    pub properties: HashMap<String, Property>,
    /// Index signatures
    pub index_signatures: Vec<IndexSignature>,
    /// Call signatures (for callable objects)
    pub call_signatures: Vec<FunctionType>,
    /// Constructor signatures (for newable objects)
    pub construct_signatures: Vec<FunctionType>,
}

/// Property of an object type
#[derive(Debug, Clone)]
pub struct Property {
    /// Property name
    pub name: String,
    /// Property type
    pub ty: Box<Type>,
    /// Is the property optional?
    pub optional: bool,
    /// Is the property readonly?
    pub readonly: bool,
}

/// Index signature ([key: string]: T)
#[derive(Debug, Clone)]
pub struct IndexSignature {
    /// Key type (string or number)
    pub key_type: Box<Type>,
    /// Value type
    pub value_type: Box<Type>,
    /// Is readonly?
    pub readonly: bool,
}

/// Function type
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for type representation
pub struct FunctionType {
    /// Type parameters
    pub type_parameters: Vec<TypeParameter>,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Return type
    pub return_type: Box<Type>,
    /// This type (for methods)
    pub this_type: Option<Box<Type>>,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: Box<Type>,
    /// Is optional?
    pub optional: bool,
    /// Is rest parameter?
    pub rest: bool,
}

/// Type parameter (generic)
#[derive(Debug, Clone)]
pub struct TypeParameter {
    /// Parameter name
    pub name: String,
    /// Constraint (extends clause)
    pub constraint: Option<Box<Type>>,
    /// Default type
    pub default: Option<Box<Type>>,
}

/// Conditional type
#[derive(Debug, Clone)]
pub struct ConditionalType {
    /// Check type
    pub check_type: Box<Type>,
    /// Extends type
    pub extends_type: Box<Type>,
    /// True branch
    pub true_type: Box<Type>,
    /// False branch
    pub false_type: Box<Type>,
}

/// Mapped type
#[derive(Debug, Clone)]
pub struct MappedType {
    /// Type parameter name
    pub type_parameter: String,
    /// Constraint type (in keyof T, etc.)
    pub constraint: Box<Type>,
    /// Template type
    pub template_type: Box<Type>,
    /// Readonly modifier (+readonly, -readonly, or none)
    pub readonly_modifier: Option<bool>,
    /// Optional modifier (+?, -?, or none)
    pub optional_modifier: Option<bool>,
}

/// Indexed access type (T[K])
#[derive(Debug, Clone)]
pub struct IndexedAccessType {
    /// Object type
    pub object_type: Box<Type>,
    /// Index type
    pub index_type: Box<Type>,
}

/// Type reference (named type)
#[derive(Debug, Clone)]
pub struct TypeReference {
    /// Type name
    pub name: String,
    /// Type arguments
    pub type_arguments: Vec<Type>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_id() {
        let id1 = TypeId::new(1);
        let id2 = TypeId::new(1);
        let id3 = TypeId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_type_flags_primitives() {
        assert!(TypeFlags::STRING.intersects(TypeFlags::PRIMITIVE));
        assert!(TypeFlags::NUMBER.intersects(TypeFlags::PRIMITIVE));
        assert!(TypeFlags::BOOLEAN.intersects(TypeFlags::PRIMITIVE));
        assert!(TypeFlags::VOID.intersects(TypeFlags::PRIMITIVE));
        assert!(TypeFlags::UNDEFINED.intersects(TypeFlags::PRIMITIVE));
        assert!(TypeFlags::NULL.intersects(TypeFlags::PRIMITIVE));
    }

    #[test]
    fn test_type_flags_literals() {
        assert!(TypeFlags::STRING_LITERAL.intersects(TypeFlags::LITERAL));
        assert!(TypeFlags::NUMBER_LITERAL.intersects(TypeFlags::LITERAL));
        assert!(TypeFlags::BOOLEAN_LITERAL.intersects(TypeFlags::LITERAL));
    }

    #[test]
    fn test_type_flags_combine() {
        let flags = TypeFlags::STRING | TypeFlags::NUMBER;
        assert!(flags.contains(TypeFlags::STRING));
        assert!(flags.contains(TypeFlags::NUMBER));
        assert!(!flags.contains(TypeFlags::BOOLEAN));
    }

    #[test]
    fn test_type_flags() {
        assert_eq!(Type::Any.flags(), TypeFlags::ANY);
        assert_eq!(Type::Unknown.flags(), TypeFlags::UNKNOWN);
        assert_eq!(Type::Never.flags(), TypeFlags::NEVER);
        assert_eq!(Type::Void.flags(), TypeFlags::VOID);
        assert_eq!(Type::String.flags(), TypeFlags::STRING);
        assert_eq!(Type::Number.flags(), TypeFlags::NUMBER);
        assert_eq!(Type::Boolean.flags(), TypeFlags::BOOLEAN);
    }

    #[test]
    fn test_type_literal_flags() {
        assert_eq!(
            Type::StringLiteral("hello".to_string()).flags(),
            TypeFlags::STRING_LITERAL
        );
        assert_eq!(Type::NumberLiteral(42.0).flags(), TypeFlags::NUMBER_LITERAL);
        assert_eq!(
            Type::BooleanLiteral(true).flags(),
            TypeFlags::BOOLEAN_LITERAL
        );
    }

    #[test]
    fn test_any_assignable_to_anything() {
        let any = Type::Any;
        assert!(any.is_assignable_to(&Type::String));
        assert!(any.is_assignable_to(&Type::Number));
        assert!(any.is_assignable_to(&Type::Boolean));
        assert!(any.is_assignable_to(&Type::Void));
    }

    #[test]
    fn test_anything_assignable_to_any() {
        let any = Type::Any;
        assert!(Type::String.is_assignable_to(&any));
        assert!(Type::Number.is_assignable_to(&any));
        assert!(Type::Boolean.is_assignable_to(&any));
    }

    #[test]
    fn test_anything_assignable_to_unknown() {
        let unknown = Type::Unknown;
        assert!(Type::String.is_assignable_to(&unknown));
        assert!(Type::Number.is_assignable_to(&unknown));
        assert!(Type::Any.is_assignable_to(&unknown));
    }

    #[test]
    fn test_never_assignable_to_everything() {
        let never = Type::Never;
        assert!(never.is_assignable_to(&Type::String));
        assert!(never.is_assignable_to(&Type::Number));
        assert!(never.is_assignable_to(&Type::Never));
    }

    #[test]
    fn test_nothing_assignable_to_never() {
        let never = Type::Never;
        assert!(!Type::String.is_assignable_to(&never));
        assert!(!Type::Number.is_assignable_to(&never));
        // Note: Any is special and is assignable to everything
        // This is by TypeScript design
    }

    #[test]
    fn test_same_primitives_assignable() {
        assert!(Type::String.is_assignable_to(&Type::String));
        assert!(Type::Number.is_assignable_to(&Type::Number));
        assert!(Type::Boolean.is_assignable_to(&Type::Boolean));
        assert!(Type::Void.is_assignable_to(&Type::Void));
        assert!(Type::Null.is_assignable_to(&Type::Null));
        assert!(Type::Undefined.is_assignable_to(&Type::Undefined));
    }

    #[test]
    fn test_different_primitives_not_assignable() {
        assert!(!Type::String.is_assignable_to(&Type::Number));
        assert!(!Type::Number.is_assignable_to(&Type::Boolean));
        assert!(!Type::Boolean.is_assignable_to(&Type::String));
    }

    #[test]
    fn test_string_literal_assignable_to_string() {
        let literal = Type::StringLiteral("hello".to_string());
        assert!(literal.is_assignable_to(&Type::String));
    }

    #[test]
    fn test_number_literal_assignable_to_number() {
        let literal = Type::NumberLiteral(42.0);
        assert!(literal.is_assignable_to(&Type::Number));
    }

    #[test]
    fn test_boolean_literal_assignable_to_boolean() {
        let literal = Type::BooleanLiteral(true);
        assert!(literal.is_assignable_to(&Type::Boolean));
    }

    #[test]
    fn test_same_string_literals_assignable() {
        let a = Type::StringLiteral("hello".to_string());
        let b = Type::StringLiteral("hello".to_string());
        assert!(a.is_assignable_to(&b));
    }

    #[test]
    fn test_different_string_literals_not_assignable() {
        let a = Type::StringLiteral("hello".to_string());
        let b = Type::StringLiteral("world".to_string());
        assert!(!a.is_assignable_to(&b));
    }

    #[test]
    fn test_array_assignability() {
        let string_array = Type::Array(Box::new(Type::String));
        let another_string_array = Type::Array(Box::new(Type::String));
        let number_array = Type::Array(Box::new(Type::Number));

        assert!(string_array.is_assignable_to(&another_string_array));
        assert!(!string_array.is_assignable_to(&number_array));
    }

    #[test]
    fn test_union_target_assignability() {
        let string_or_number = Type::Union(vec![Type::String, Type::Number]);

        // string is assignable to string | number
        assert!(Type::String.is_assignable_to(&string_or_number));
        // number is assignable to string | number
        assert!(Type::Number.is_assignable_to(&string_or_number));
        // boolean is NOT assignable to string | number
        assert!(!Type::Boolean.is_assignable_to(&string_or_number));
    }

    #[test]
    fn test_union_source_assignability() {
        let string_or_number = Type::Union(vec![Type::String, Type::Number]);

        // string | number is NOT assignable to string alone
        assert!(!string_or_number.is_assignable_to(&Type::String));
    }

    #[test]
    fn test_undefined_assignable_to_void() {
        assert!(Type::Undefined.is_assignable_to(&Type::Void));
    }

    #[test]
    fn test_object_type_default() {
        let obj = ObjectType::default();
        assert!(obj.properties.is_empty());
        assert!(obj.index_signatures.is_empty());
        assert!(obj.call_signatures.is_empty());
        assert!(obj.construct_signatures.is_empty());
    }

    #[test]
    fn test_property_creation() {
        let prop = Property {
            name: "test".to_string(),
            ty: Box::new(Type::String),
            optional: false,
            readonly: false,
        };

        assert_eq!(prop.name, "test");
        assert!(!prop.optional);
        assert!(!prop.readonly);
    }

    #[test]
    fn test_function_type() {
        let func = FunctionType {
            type_parameters: Vec::new(),
            parameters: vec![Parameter {
                name: "x".to_string(),
                ty: Box::new(Type::Number),
                optional: false,
                rest: false,
            }],
            return_type: Box::new(Type::String),
            this_type: None,
        };

        assert_eq!(func.parameters.len(), 1);
        assert_eq!(func.parameters[0].name, "x");
    }

    #[test]
    fn test_type_parameter() {
        let param = TypeParameter {
            name: "T".to_string(),
            constraint: Some(Box::new(Type::String)),
            default: None,
        };

        assert_eq!(param.name, "T");
        assert!(param.constraint.is_some());
        assert!(param.default.is_none());
    }

    #[test]
    fn test_type_reference() {
        let reference = TypeReference {
            name: "Array".to_string(),
            type_arguments: vec![Type::String],
        };

        assert_eq!(reference.name, "Array");
        assert_eq!(reference.type_arguments.len(), 1);
    }

    #[test]
    fn test_intersection_assignability() {
        let a_and_b = Type::Intersection(vec![
            Type::Object(ObjectType::default()),
            Type::Object(ObjectType::default()),
        ]);

        // For now, basic test - full implementation would check properties
        assert!(Type::Any.is_assignable_to(&a_and_b));
    }
}
