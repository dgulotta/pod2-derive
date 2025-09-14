#[cfg(test)]
mod test {
    use pod2_derive::TryFromValue;

    #[derive(TryFromValue)]
    #[allow(dead_code)]
    struct MyStruct {
        a: i64,
        b: i64,
        c: Vec<i64>,
        d: HashSet<i64>,
    }

    use std::collections::{HashMap, HashSet};

    use pod2::middleware::{
        Key, Params, Value,
        containers::{Array, Dictionary, Set},
    };

    #[test]
    fn test_tfv() {
        let arr = Array::new(6, vec![Value::from(3), Value::from(4)]).unwrap();
        let set = Set::new(6, [5, 6].into_iter().map(Value::from).collect()).unwrap();
        let mut kvs = HashMap::new();
        kvs.insert(Key::from("a"), Value::from(1));
        kvs.insert(Key::from("b"), Value::from(2));
        kvs.insert(Key::from("c"), Value::from(arr));
        kvs.insert(Key::from("d"), Value::from(set));
        let d = Dictionary::new(Params::default().max_depth_mt_containers, kvs).unwrap();
        let v = Value::from(d);
        let _s: MyStruct = v.try_into().unwrap();
    }
}
