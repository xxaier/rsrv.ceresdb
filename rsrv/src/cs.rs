use csdb::{conn_by_env, Db, SQL};
use lazy_static::lazy_static;

lazy_static! {
  pub static ref CS: Db = conn_by_env("CERESDB_GRPC").unwrap();
  // 插入时间/用户操作时间/用户id/操作/对象类型/对象id
  pub static ref FAV_INSERT: SQL =
    CS.sql("INSERT INTO fav (ts,ctime,uid,action,kind,rid) VALUES ({},{},{},{},{},{})");
}
