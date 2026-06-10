/// Trait for module deploy data with preflight serialization validation.
///
/// Every module that can be instantiated during `deploy_on()` implements
/// this on its deploy data type. The default `preflight()` method
/// round-trips through JSON to catch serialization issues before they
/// hit the chain.
pub trait DaoDeployData: Clone {
    /// The contract's instantiate message type.
    type Init: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug;

    /// Convert this deploy data into the contract's instantiate message.
    fn into_init(self) -> Self::Init;

    /// Preflight validation: serialize → deserialize round-trip.
    /// Called automatically before instantiation in `deploy_on()`.
    fn preflight(&self) -> Result<(), String>
    where
        Self: Sized,
    {
        let msg = self.clone().into_init();
        let json =
            serde_json::to_vec(&msg).map_err(|e| format!("serialize failed: {e}"))?;
        let _: Self::Init =
            serde_json::from_slice(&json).map_err(|e| format!("deserialize failed: {e}"))?;
        Ok(())
    }
}
