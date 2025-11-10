use rstest::rstest;
use serde::Serialize;
use serde_json::json;
use serde_more::SerializeMore;
use serde_with::serde_as;
use testresult::TestResult;

#[derive(SerializeMore)]
#[more(key = "normal_field_squared", value = "squared")]
struct Basic {
    normal_field: u32,
}

impl Basic {
    fn squared(&self) -> u32 {
        self.normal_field.pow(2)
    }
}

#[derive(SerializeMore)]
#[more(key = "x_1", value = "plus_one")]
#[more(key = "x_2", value = "plus_two")]
struct Multi {
    x: u8,
}

impl Multi {
    fn plus_one(&self) -> u8 {
        self.x + 1
    }
    fn plus_two(&self) -> u8 {
        self.x + 2
    }
}

#[derive(SerializeMore)]
#[serde(rename_all = "kebab-case")]
#[more(k = "extraVal", v = "extra_val")]
struct WithSerdeAttrs {
    field_name: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    opt_value: Option<u8>,
}

impl WithSerdeAttrs {
    fn extra_val(&self) -> &'static str {
        "ok"
    }
}

#[serde_as]
#[derive(SerializeMore)]
#[more(k = "payload_len")]
struct WithSerdeAsAttrs {
    #[serde_as(as = "serde_with::hex::Hex")]
    payload: Vec<u8>,
}

impl WithSerdeAsAttrs {
    fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

#[rstest]
#[case::struct_single(&Basic { normal_field: 7 }, json!({"normal_field":7, "normal_field_squared":49}))]
#[case::struct_multiple(&Multi { x: 3 }, json!({"x":3,"x_1":4,"x_2":5}))]
#[case::serde_attrs(&WithSerdeAttrs { field_name: 1, opt_value: None }, json!({"field-name":1, "extraVal":"ok"}))]
#[case::serde_with(&WithSerdeAsAttrs { payload: vec![0x0a, 0xff] }, json!({"payload":"0aff","payload_len":2}))]
fn serialize_more<T: Serialize>(
    #[case] input: T,
    #[case] expected: serde_json::Value,
) -> TestResult {
    let v = serde_json::to_value(input)?;
    assert_eq!(v, expected);
    Ok(())
}
