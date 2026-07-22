use anyhow::Result;

// ⟦𓁎𓍌𓍩𓍺⟧ run :: auto-generated pointer for public function run
pub fn run() -> Result<()> {
    let store = crate::store::find_current_store()?;
    let _lock = crate::store::lock_store(&store)?;
    let new_ver = crate::store::bump_version(&store)?;
    eprintln!("Version bumped to {}", new_ver);
    Ok(())
}
