use std::path::Path;
use rustorm::{Pool, EntityManager};
use crate::error::WallerError;


static KEYPAIR_STORE_TABLE: &'static str = r#"

"#;

pub struct KeypairStore {
    path: String,
    em: Option<EntityManager>,
}

impl KeypairStore{
    pub fn new(path: String) -> Self {
        Self{
            path,
            em: None,
        }
    }
 
    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }

    pub fn init(&mut self, pool: &mut Pool) -> Result<(), WallerError> {
        let mut dm = pool.dm(&self.path).map_err(|_| WallerError::DatabaseOpenError)?;
        dm.execute_sql_with_return(KEYPAIR_STORE_TABLE, &vec![]);
        
        self.open(pool)
    }

    ///
    /// url 形如 sqlite:///home/lee/rustorm/file.db
    pub fn open(&mut self, pool: &mut Pool) -> Result<(), WallerError> {
        self.em = Some(pool.em(&self.path).map_err(|_| WallerError::DatabaseOpenError)?);

        Ok(())
    }
    
    pub fn has_cert_registered(&self) -> bool {
        false
    }

    pub fn register(&self) -> Result<(), WallerError> {
        Ok(())
    }
}

