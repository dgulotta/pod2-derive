#[cfg(test)]
#[allow(dead_code)]
mod test {

    mod try_from_value {

        use pod2_derive::{IntoValue, TryFromValue};

        use std::collections::{HashMap, HashSet};

        use pod2::middleware::{
            Key, Params, Value,
            containers::{Array, Dictionary, Set},
        };

        #[derive(TryFromValue)]
        struct Struct {
            a: i64,
            b: i64,
            c: Vec<i64>,
            d: HashSet<i64>,
        }

        #[derive(TryFromValue)]
        struct UnitStruct;

        #[derive(TryFromValue)]
        struct UnitFieldStruct {}

        #[derive(TryFromValue)]
        struct UnitTupleStruct();

        #[derive(TryFromValue)]
        enum EmptyEnum {}

        #[derive(TryFromValue, PartialEq, Eq, Debug)]
        enum SimpleEnum {
            A,
            B,
        }

        #[derive(TryFromValue, PartialEq, Eq, Debug)]
        enum FancyEnum {
            A,
            B(i64),
            C { c: i64 },
        }

        #[derive(TryFromValue)]
        #[allow(dead_code)]
        struct TupleStruct(i64, i64);

        #[derive(IntoValue, TryFromValue, PartialEq, Eq, Debug, Clone, Copy)]
        struct NewtypeStructInt(i64);

        #[test]
        fn test_empty_enum() {
            assert!(EmptyEnum::try_from(Value::from(0)).is_err());
        }

        #[test]
        fn test_unit_struct() {
            let _: UnitStruct = Value::from(0).try_into().unwrap();
            let _: UnitFieldStruct = Value::from(0).try_into().unwrap();
            let _: UnitTupleStruct = Value::from(0).try_into().unwrap();
        }

        #[test]
        fn test_tuple_struct() {
            let arr = Array::new(6, vec![Value::from(0), Value::from(1)]).unwrap();
            let v = Value::from(arr);
            let _: TupleStruct = v.try_into().unwrap();
        }

        #[test]
        fn test_tuple_struct_wrong_arity() {
            let short_arr = Array::new(6, vec![Value::from(0)]).unwrap();
            let short_v = Value::from(short_arr);
            let long_arr =
                Array::new(6, vec![Value::from(0), Value::from(1), Value::from(2)]).unwrap();
            let long_v = Value::from(long_arr);
            assert!(TupleStruct::try_from(short_v).is_err());
            assert!(TupleStruct::try_from(long_v).is_err());
        }

        #[test]
        fn test_struct() {
            let arr = Array::new(6, vec![Value::from(3), Value::from(4)]).unwrap();
            let set = Set::new(6, [5, 6].into_iter().map(Value::from).collect()).unwrap();
            let mut kvs = HashMap::new();
            kvs.insert(Key::from("a"), Value::from(1));
            kvs.insert(Key::from("b"), Value::from(2));
            kvs.insert(Key::from("c"), Value::from(arr));
            kvs.insert(Key::from("d"), Value::from(set));
            let d = Dictionary::new(Params::default().max_depth_mt_containers, kvs).unwrap();
            let v = Value::from(d);
            let _: Struct = v.try_into().unwrap();
        }

        #[test]
        fn test_simple_enum() {
            let a = SimpleEnum::try_from(Value::from("A")).unwrap();
            assert_eq!(a, SimpleEnum::A);
            assert!(SimpleEnum::try_from(Value::from("C")).is_err())
        }

        #[test]
        fn test_newtype_struct() {
            let x = NewtypeStructInt(2);
            let v: Value = x.into();
            let y = NewtypeStructInt::try_from(v).unwrap();
            assert_eq!(x, y);
        }
    }

    mod from_value {
        use pod2::middleware::Value;
        use pod2_derive::FromValue;

        #[derive(FromValue)]
        struct UnitStruct;

        #[derive(FromValue)]
        struct TupleStruct0();

        #[derive(FromValue)]
        struct EmptyStruct {}

        #[derive(FromValue, PartialEq, Eq)]
        struct NewtypeStruct(Value);

        #[test]
        fn test_from_value() {
            let v = Value::from(2);
            let n = NewtypeStruct::from(v.clone());
            assert_eq!(v, n.0);
        }
    }
}
