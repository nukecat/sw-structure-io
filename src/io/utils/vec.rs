pub(crate) trait TryIntoVecExt<T: Copy>: AsRef<[T]> {
    fn try_into_vec<U>(&self) -> Result<Vec<U>, U::Error>
    where   U: TryFrom<T> {
        self.as_ref()
        .iter()
        .map(|&v| U::try_from(v))
        .collect()
    }
}

pub(crate) trait IntoVecExt<T: Copy>: IntoIterator<Item = T> + Sized + AsRef<[T]> {
    fn into_vec<U>(&self) -> Vec<U>
    where
    U: From<T>
    {
        self
        .as_ref()
        .iter()
        .map(|&v| U::from(v))
        .collect()
    }
}

pub(crate) trait IntoVecLossyExt<T: Copy>: IntoIterator<Item = T> + Sized + AsRef<[T]> {
    fn into_vec_lossy<U>(&self) -> Vec<U>
    where
    U: TryFrom<T>,
    U::Error: std::error::Error + 'static
    {
        self
        .as_ref()
        .iter()
        .filter_map(|&v| U::try_from(v).ok())
        .collect()
    }
}

impl<T: Copy> TryIntoVecExt<T> for Vec<T> {}

impl<T: Copy> IntoVecExt<T> for Vec<T> {}

impl<T: Copy> IntoVecLossyExt<T> for Vec<T> {}
