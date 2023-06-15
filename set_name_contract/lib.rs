#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod name_setter {
    use ink::prelude::string::String;

    #[ink(storage)]
    pub struct NameSetter {
        name: String,
    }

    impl NameSetter {
        /// Constructor that initializes the `name` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(init_value: String) -> Self {
            Self { name: init_value }
        }

        /// Constructor that initializes the `name` value to the empty string.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// Set the `name` to the given value.
        #[ink(message)]
        pub fn set_name(&mut self, new_value: String) {
            self.name = new_value;
        }

        /// terminate the contract
        #[ink(message)]
        pub fn terminate(&mut self) {
            self.env().terminate_contract(self.env().caller());
        }

        /// Simply returns the current `name` value.
        #[ink(message)]
        pub fn get_name(&self) -> String {
            self.name.clone()
        }
    }
}
