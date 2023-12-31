use csdb::{conn_by_env, Db, SQL};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DB: Db = conn_by_env("CERESDB_GRPC").unwrap();
    pub static ref SQL_DROP_FAV: SQL = DB.sql("DROP TABLE fav");

    // 插入时间/用户操作时间/用户id/操作/对象类型/对象id
    pub static ref SQL_FAV: SQL = DB.sql(r#"CREATE TABLE fav (
  uid uint64 NOT NULL,
  ctime TIMESTAMP NOT NULL,
  action uint8 NOT NULL,
  cid uint8 NOT NULL,
  rid uint64 NOT NULL,
  TIMESTAMP KEY(ts),
  PRIMARY KEY(uid, ctime)
) ENGINE=Analytic WITH (
  compression='ZSTD',
  enable_ttl='false'
)"#);
    pub static ref SQL_INSERT: SQL = DB.sql( "INSERT INTO fav (uid,ctime,action,cid,rid) VALUES ({},{},{},{},{})");
    pub static ref SQL_SELECT: SQL = DB.sql( "SELECT * FROM fav");
    // pub static ref SQL_DELETE: SQL = DB.sql(, "DELETE FROM fav WHERE ts={} AND uid={}");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    loginit::init();

    let _ = SQL_DROP_FAV.exe(()).await;
    SQL_FAV.exe(()).await?;
    // SQL_INSERT.exe((1, 2, 3, 4, 5, 6)).await?;
    // SQL_INSERT.exe((2, 2, 3, 4, 5, 6)).await?;

    let li = SQL_SELECT.li(()).await?;
    for i in li {
        dbg!(&i);
    }
    // SQL_DELETE.exe([1, 3]).await?;
    Ok(())
}
