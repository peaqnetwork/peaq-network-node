//! This module containes trait definitions and implementations to describe the averaging.
//! 
//! # Averaging
//! 
//! ## General Term Description
//! 
//! By averaging the general ability is meant, to figure out somehow an average-value of some
//! arbitrary data series. For example, a pallet that is generating a varying value on each
//! finalized block, could provide such an average-value of that series. Because it is not
//! possible to calculate that true mathematical average/mean totally accurate, due to the
//! indefinite need of memory in a blockchain, there arises the need for an estimation or
//! approximation of that average/mean-value. Any pallet or arbitrary implementation that
//! provides such an approximation/estimation of an average-value can be marked by the following
//! trait definitions.
//! 
//! By using these traits, you can connect pallets together, without using a direct coupling
//! of those pallets. Pallets, which are using these trait definitions can easily be exchanged,
//! only by switching them in the runtime-implementation, and without modifying or adapting
//! the existing/remaining ones in the runtime.
//! 
//! ## Abstract Trait Design
//! 
//! There might be a pallet, which shall provide more than one average-value of a data series.
//! There might also be a pallet, that shall provide the average-value for different recipients,
//! e.g. if the final result also depends on another parameter.
//! 
//! To map these two requirements and the ability to keep it simple, in case you will provide
//! just one average-value, there are four different trait definitions. You may implement only
//! one of them, or even all of them.
//! 
//! By using generics for defining the datatype on return values, the optional recipients, and
//! the optional selector of the average-value, you can individually adapt the use case and make
//! them fit to your special use case. 
//! 
//! ## Example Situations
//! 
//! 1. The pallet/object will provide just one single average-value, that does not depend on any
//!    recipient, just implement `ProvidesAverage<T>` for your pallet/object.
//! 
//! 2. The pallet/object will provide multiple average-values with same return type, define some
//!    selector-type, that is suitable for you and implement the `ProvidesAverages<T, S>`:
//!    ```ignore
//!    pub enum AverageSelector {
//!        AverageA,
//!        AverageB,
//!    }
//! 
//!    impl<T> ProvidesAverages<u32, AverageSelector> for Pallet<T> {
//!        // ...
//!    }
//!    ```
//!    To connect another pallet to this interface, without creating a coupling, you can add the
//!    need for the trait within the pallet's config definition and a getter for the right selector-
//!    type:
//!    ```ignore
//!    type AvgBlockRewardProvider: ProvidesAverageFor<Self::CurrencyBalance, Self::AvgRecipientSelector>;
//!    
//!    type AvgRecipientSelector: Parameter;
//!    
//!    #[pallet::constant]
//!    type AvgBlockRewardRecipient: Get<Self::AvgRecipientSelector>;
//!    ```


/// An object which provides an average-value on a varying paramter.
pub trait ProvidesAverage<T> {
    /// The getter for the average-value
    fn get_average() -> T;
}

/// Ability to provide multiple average-values on different varying paramters.
pub trait ProvidesAverages<T, S> {
    /// Getter method with selector between different average-values.
    fn get_average_by(sel: S) -> T;
}

/// Same as ProvidesAverage but with a recipient.
pub trait ProvidesAverageFor<T, R> {
    /// The getter for the average-value with selector for recipient.
    fn get_average_for(r: R) -> T;
}

/// Same as ProvidesAverages but with a recipient.
pub trait ProvidesAveragesFor<T, S, R> {
    /// Getter method with selector between different average-values and selector for recipient.
    fn get_average_for_by(sel: S, rec: R) -> T;
}
