#[cfg(test)]
mod test {
    use pod2_derive::TryFromValue;

    #[derive(TryFromValue)]
    #[allow(dead_code)]
    struct Struct {
        a: i64,
        b: i64,
        c: Vec<i64>,
        d: HashSet<i64>,
    }

    #[derive(TryFromValue)]
    struct UnitStruct;

    #[derive(TryFromValue)]
    struct TupleStructWithNoFields();

    #[derive(TryFromValue)]
    enum EmptyEnum {}

    #[derive(TryFromValue)]
    #[allow(dead_code)]
    struct MyTupleStruct(i64, i64);

    use std::collections::{HashMap, HashSet};

    use pod2::middleware::{
        Key, Params, Value,
        containers::{Array, Dictionary, Set},
    };

    #[test]
    fn test_tfv_struct() {
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
    fn test_tfv_empty_enum() {
        assert!(EmptyEnum::try_from(Value::from(0)).is_err());
    }

    #[test]
    fn test_tfv_unit_struct() {
        let _: UnitStruct = Value::from(0).try_into().unwrap();
        let _: TupleStructWithNoFields = Value::from(0).try_into().unwrap();
    }

    #[test]
    fn test_tfv_tuple_struct() {
        let arr = Array::new(6, vec![Value::from(0), Value::from(1)]).unwrap();
        let v = Value::from(arr);
        let _: MyTupleStruct = v.try_into().unwrap();
    }

    #[test]
    fn test_tfv_tuple_struct_wrong_arity() {
        let short_arr = Array::new(6, vec![Value::from(0)]).unwrap();
        let short_v = Value::from(short_arr);
        let long_arr = Array::new(6, vec![Value::from(0), Value::from(1), Value::from(2)]).unwrap();
        let long_v = Value::from(long_arr);
        assert!(MyTupleStruct::try_from(short_v).is_err());
        assert!(MyTupleStruct::try_from(long_v).is_err());
    }
}
