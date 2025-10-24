pub mod modules;

pub mod utils {
    use uuid::Uuid;

    /// Generate a 4-character identification using first 4 characters of a random UUID
    pub fn generate_random_identification(uuid: Uuid) -> String {
        let uuid_str = uuid.to_string().replace('-', "");

        uuid_str.chars().take(4).collect::<String>().to_uppercase()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_generate_identification_format() {
            let uuid = Uuid::new_v4();
            let id = generate_random_identification(uuid);

            // Should be exactly 4 characters
            assert_eq!(id.len(), 4);

            // Should contain only alphanumeric characters
            assert!(id.chars().all(|c| c.is_alphanumeric()));

            // Should be uppercase
            assert_eq!(id, *id.to_uppercase());
        }
    }
}
