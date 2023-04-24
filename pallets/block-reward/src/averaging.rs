//! This module containes trait definitions and implementations to describe the averaging.


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
