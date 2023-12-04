#[macro_export]
macro_rules! home_path {
    ($my_struct:ident, $path:literal) => {
        #[derive(Clone, Debug)]
        pub struct $my_struct(String);

        impl Default for $my_struct {
            fn default() -> Self {
                match dirs::home_dir() {
                    None => {
                        println!("$HOME doesn't exist. This probably won't do what you want.");
                        $my_struct(".".to_string())
                    }
                    Some(mut path) => {
                        path.push($path);
                        $my_struct(path.as_path().display().to_string())
                    }
                }
            }
        }

        impl ToString for $my_struct {
            fn to_string(&self) -> String {
                self.0.clone()
            }
        }

        impl FromStr for $my_struct {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }
    };
}

#[macro_export]
macro_rules! patch_type_layout {
    ($idl:expr, $typename:expr, $struct:ty, $category:expr) => {
        let target_type = $idl[$category]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|el| el["name"] == $typename)
        .unwrap();

    let target_type_layout = <$struct>::type_layout();
    let idl_fields = target_type["type"]["fields"].as_array_mut().unwrap();

        let mut padding_field_counter = 0;
        for (index, field) in target_type_layout.fields.iter().enumerate() {
            match field {
                type_layout::Field::Field { .. } => {}
                type_layout::Field::Padding { size } => {
                    let padding_field = serde_json::json!(
                        {
                            "name": format!("auto_padding_{}", padding_field_counter),
                            "type": {
                                "array": ["u8", size]
                            }
                        }
                    );
                    idl_fields.insert(index, padding_field);
                    padding_field_counter += 1;
                }
            }
        }
    };
}
