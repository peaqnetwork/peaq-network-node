//! This module containes trait definitions and implementations to describe the averaging.


/// An object which provides an average-value on a varying paramter.
pub trait ProvidesAverage {
    type Type;

    /// The getter for the average-value
    fn get_average(&self) -> Self::Type;
}


/// Ability to provide multiple average-values on different varying paramters.
pub trait ProvidesAverages {
    type Type;
    type Selector;

    fn get_average_provider(sel: Self::Selector) -> Box<dyn ProvidesAverage<Type = Self::Type>>;
}


// TODO later, probably we need a proc-macro here
// /// Macro declarates a single pallet-storage for an average-value.
// /// Parameters the identifier (which gots extended by 'Avg' at the beginning), the
// /// name of the getter function, and the datatype of the average-value to be stored.
// #[macro_export]
// macro_rules! average_storage_decl {
//     ($name:ident, $getter:ident, $type:ty) => {
//         #[pallet::storage]
// 	    #[pallet::getter(fn $getter)]
// 	    pub(crate) type $name<T: Config> = StorageValue<_, $type, ValueQuery>;
//     };
// }
