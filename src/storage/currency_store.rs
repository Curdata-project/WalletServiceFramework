use std::path::Path;
use rustorm::{Pool, EntityManager};
use crate::error::WallerError;


static CURRENCY_STORE_TABLE: &'static str = r#"
CREATE TABLE "currency" (
    "id" VARCHAR(255) NOT NULL,
    "quota_control_field" TEXT NOT NULL,
    "explain_info" TEXT NOT NULL,
    "state" VARCHAR(255) NOT NULL,
    "owner" VARCHAR(255) NOT NULL,
    "create_time" TIMESTAMP NOT NULL,
    "update_time" TIMESTAMP NOT NULL,
    PRIMARY KEY ("id")
  )
"#;

pub struct CurrencyStore {
    path: String,
    em: Option<EntityManager>,
}

impl CurrencyStore{
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
        dm.execute_sql_with_return(CURRENCY_STORE_TABLE, &vec![]);
        
        self.open(pool)
    }

    ///
    /// url 形如 sqlite:///home/lee/rustorm/file.db
    pub fn open(&mut self, pool: &mut Pool) -> Result<(), WallerError> {
        self.em = Some(pool.em(&self.path).map_err(|_| WallerError::DatabaseOpenError)?);

        Ok(())
    }
}

