use lctx_model::domain::{value::Literal, *};

#[test]
fn unicode_values_preserve_nul_and_reject_non_utf8_binary() {
    let model = model().unwrap();
    let mut expected: Vec<_> = ["", "ordinary", "\0", "é\0終"]
        .into_iter()
        .map(|text| Literal::String { value: text.into() })
        .collect();
    expected.sort_by_key(Record::id);
    let batch = Batch::new(&model, expected.clone(), &budget()).unwrap();
    assert_eq!(Literal::decode(batch.arrow()).unwrap(), expected);
    let text = Utf8Text::from("é\0終");
    use serde::Deserialize;
    use serde::de::value::{BorrowedBytesDeserializer, Error};
    assert_eq!(
        Utf8Text::deserialize(BorrowedBytesDeserializer::<Error>::new(text.as_bytes())).unwrap(),
        text
    );
    assert!(Utf8Text::deserialize(BorrowedBytesDeserializer::<Error>::new(&[255])).is_err());
    let mut plain = KeySink::new("unicode-value");
    "é\0終".to_owned().encode(&mut plain);
    let mut nominal = KeySink::new("unicode-value");
    text.encode(&mut nominal);
    assert_eq!(plain.finish(), nominal.finish());
    assert_ne!(
        Literal::String { value: "\0".into() }.id(),
        Literal::Bytes {
            value: EvidenceBytes(vec![0])
        }
        .id()
    );
}

fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(1 << 30).unwrap()
}
