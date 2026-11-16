use super::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Snapshot {
    format: u32,
    space: String,
    heads: Vec<String>,
    document: Vec<u8>,
}

fn identity(doc: &Automerge) -> Result<String> {
    let nodes = view(doc)?;
    if !nodes.contains_key("s0")
        || doc.get(ROOT, "single_space_fixture")?.is_none()
        || nodes.values().filter(|n| n.kind == "space").count() != 1
        || scope(&nodes, "s0") != nodes
    {
        return Err("Snapshot requires exactly one complete synthetic space".into());
    }
    Ok(nodes["s0"].id.to_string())
}

impl Snapshot {
    fn encode(&self) -> Result<Vec<u8>> {
        let metadata =
            serde_json::to_vec(&(self.format, &self.space, &self.heads, self.document.len()))?;
        let length = u32::try_from(metadata.len())?;
        let mut bytes = length.to_le_bytes().to_vec();
        bytes.extend(metadata);
        bytes.extend(&self.document);
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let prefix: [u8; 4] = bytes
            .get(..4)
            .ok_or("Truncated snapshot header")?
            .try_into()?;
        let length = u32::from_le_bytes(prefix) as usize;
        let end = 4_usize
            .checked_add(length)
            .ok_or("Invalid snapshot length")?;
        let metadata = bytes.get(4..end).ok_or("Truncated snapshot metadata")?;
        let (format, space, heads, document_length): (u32, String, Vec<String>, usize) =
            serde_json::from_slice(metadata)?;
        let document = bytes.get(end..).ok_or("Missing snapshot document")?;
        if document.len() != document_length {
            return Err("Snapshot document length mismatch".into());
        }
        Ok(Self {
            format,
            space,
            heads,
            document: document.to_vec(),
        })
    }

    fn capture(doc: &Automerge) -> Result<Self> {
        let snapshot = Self {
            format: 1,
            space: identity(doc)?,
            heads: doc.get_heads().iter().map(ToString::to_string).collect(),
            document: doc.save_and_verify()?,
        };
        snapshot.load()?;
        Ok(snapshot)
    }

    fn load(&self) -> Result<Automerge> {
        if self.format != 1 {
            return Err("Unsupported internal probe snapshot format".into());
        }
        let doc = Automerge::load(&self.document)?;
        let heads: Vec<_> = doc.get_heads().iter().map(ToString::to_string).collect();
        if heads != self.heads || identity(&doc)? != self.space {
            return Err("Snapshot identity or heads do not match".into());
        }
        history::versions(&doc)?;
        Ok(doc)
    }
}

fn initialize(store: &mut Store) -> Result<()> {
    store.0.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots(name TEXT PRIMARY KEY, payload BLOB NOT NULL);
         CREATE TABLE IF NOT EXISTS lifecycle(id TEXT PRIMARY KEY, paused INTEGER NOT NULL);
         INSERT OR IGNORE INTO lifecycle VALUES ('A', 0);",
    )?;
    Ok(())
}

fn save_snapshot(store: &mut Store, name: &str, doc: &Automerge) -> Result<()> {
    let snapshot = Snapshot::capture(doc)?;
    store.0.execute(
        "INSERT INTO snapshots VALUES (?1, ?2)",
        params![name, snapshot.encode()?],
    )?;
    Ok(())
}

fn read_snapshot(store: &Store, name: &str) -> Result<Snapshot> {
    let bytes: Vec<u8> =
        store
            .0
            .query_row("SELECT payload FROM snapshots WHERE name=?1", [name], |r| {
                r.get(0)
            })?;
    let snapshot = Snapshot::decode(&bytes)?;
    snapshot.load()?;
    Ok(snapshot)
}

fn delete_snapshot(store: &mut Store, name: &str) -> Result<()> {
    if store
        .0
        .execute("DELETE FROM snapshots WHERE name=?1", [name])?
        != 1
    {
        return Err("Unknown snapshot".into());
    }
    Ok(())
}

fn registers(n: &Node) -> BTreeMap<String, Vec<Variant>> {
    let mut fields = n.fields.clone();
    for (key, value) in [
        ("life", &n.life),
        ("cycles", &n.cycles),
        ("space_group", &n.space_group),
        ("column_group", &n.column_group),
    ] {
        fields.insert(key.into(), value.clone());
    }
    fields
}

fn restored(current: &Automerge, snapshot: &Snapshot) -> Result<Automerge> {
    let source = snapshot.load()?;
    if identity(current)? != snapshot.space {
        return Err("Snapshot belongs to another space".into());
    }
    current.fork_at(&source.get_heads())?;
    let old = view(&source)?;
    let known = view(current)?;
    for (key, n) in &old {
        if known.get(key).map(|n| &n.id) != Some(&n.id) {
            return Err("Snapshot graph is not an ancestor of this replica".into());
        }
    }
    let mut candidate = current.clone();
    let mut branches = vec![];
    let mut remap = BTreeMap::new();
    let mut changed = Vec::new();
    for (key, n) in &old {
        let current_fields = registers(&known[key]);
        for (field, mut desired) in registers(n) {
            let mut existing = current_fields[&field].clone();
            desired.sort_by(|a, b| (&a.value, &a.operation).cmp(&(&b.value, &b.operation)));
            existing.sort_by(|a, b| (&a.value, &a.operation).cmp(&(&b.value, &b.operation)));
            if desired
                .iter()
                .map(|v| &v.value)
                .eq(existing.iter().map(|v| &v.value))
            {
                for (source, current) in desired.iter().zip(&existing) {
                    remap.insert(source.operation.clone(), current.operation.clone());
                }
            } else {
                changed.push((n.id.clone(), field, desired));
            }
        }
    }
    for (id, field, variants) in &changed {
        for variant in variants.iter().skip(1) {
            let mut branch = current.fork();
            let mut tx = branch.transaction();
            let assignment = put(&mut tx, id, field, &variant.value)?;
            remap.insert(variant.operation.clone(), assignment.to_string());
            tx.commit();
            branches.push(branch);
        }
    }
    let history_root = candidate.get(ROOT, "history")?.ok_or("Missing history")?.1;
    let history_len = candidate.length(&history_root);
    let source_events: Vec<_> = old.values().flat_map(|n| n.barriers.keys()).collect();
    let current_events: Vec<_> = known.values().flat_map(|n| n.barriers.keys()).collect();
    let mut tx = candidate.transaction();
    let action = tx.put_object(ROOT, "restoration", ObjType::Map)?;
    tx.put(
        action,
        "source_heads",
        serde_json::to_string(&snapshot.heads)?,
    )?;
    for (key, n) in &known {
        if !old.contains_key(key) {
            put(&mut tx, &n.id, "life", "purged")?;
        }
    }
    for (id, field, variants) in &changed {
        let v = &variants[0];
        let assignment = put(&mut tx, id, field, &v.value)?;
        remap.insert(v.operation.clone(), assignment.to_string());
    }
    for n in old.values() {
        let ack = tx
            .get(&n.id, "acknowledged")?
            .ok_or("Missing acknowledgments")?
            .1;
        for event in &source_events {
            if !n.acknowledged.contains(event) {
                tx.delete(&ack, event.as_str())?;
            }
        }
        for event in &current_events {
            if !source_events.contains(event) || n.acknowledged.contains(event) {
                tx.put(&ack, event.as_str(), true)?;
            }
        }
    }
    for index in (0..history_len).rev() {
        tx.delete(&history_root, index)?;
    }
    for (index, version) in history::versions(&source)?.iter().enumerate() {
        let entry = tx.insert_object(&history_root, index, ObjType::Map)?;
        for (field, value) in [
            ("card", &version.card),
            ("title", &version.title),
            ("text", &version.text),
            (
                "title_op",
                remap.get(&version.title_op).unwrap_or(&version.title_op),
            ),
            (
                "text_op",
                remap.get(&version.text_op).unwrap_or(&version.text_op),
            ),
        ] {
            tx.put(&entry, field, value.as_str())?;
        }
    }
    tx.commit();
    for mut branch in branches {
        candidate.merge(&mut branch)?;
    }
    candidate.load_incremental(&snapshot.document)?;
    Ok(candidate)
}

fn restore(store: &mut Store, name: &str, observed: &Value) -> Result<()> {
    let tx = store
        .0
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (document, actor, peer): (Vec<u8>, Vec<u8>, Vec<u8>) = tx.query_row(
        "SELECT document,actor,peer FROM replicas WHERE id='A'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut doc = Automerge::load(&document)?;
    doc.set_actor(actor.into());
    let mut a = Replica {
        doc,
        peer: State::decode(&peer)?,
    };
    if a.fingerprint()? != *observed {
        return Err("Stale restore: inspect current state again".into());
    }
    let bytes: Vec<u8> =
        tx.query_row("SELECT payload FROM snapshots WHERE name=?1", [name], |r| {
            r.get(0)
        })?;
    let snapshot = Snapshot::decode(&bytes)?;
    let safety = Snapshot::capture(&a.doc)?;
    a.doc = restored(&a.doc, &snapshot)?;
    tx.execute(
        "INSERT INTO snapshots VALUES ('before-restore', ?1)
        ON CONFLICT(name) DO UPDATE SET payload=excluded.payload",
        [safety.encode()?],
    )?;
    Store::write_in(&tx, "A", &a)?;
    if tx.execute("UPDATE lifecycle SET paused=1 WHERE id='A'", [])? != 1 {
        return Err("Missing local pause state".into());
    }
    tx.commit()?;
    Ok(())
}

fn paused(store: &Store) -> Result<bool> {
    Ok(store
        .0
        .query_row("SELECT paused FROM lifecycle WHERE id='A'", [], |r| {
            r.get(0)
        })?)
}

fn resume(store: &mut Store) -> Result<()> {
    store
        .0
        .execute("UPDATE lifecycle SET paused=0 WHERE id='A'", [])?;
    Ok(())
}

fn sync(store: &mut Store) -> Result<()> {
    if paused(store)? {
        return Err("Space exchange paused after restoration; explicitly resume".into());
    }
    exchange(store)
}

fn seed(store: &mut Store) -> Result<()> {
    initialize(store)?;
    let mut a = fixture(false)?;
    history::save_pair(&mut a.doc, "k0", "Снимок", "Исходная история 😀")?;
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    sync(store)?;
    save_snapshot(store, "initial", &a.doc)
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    if phase.starts_with("lifecycle-compact-") {
        return maintenance::run(directory, phase);
    }
    if phase.starts_with("lifecycle-measure-") {
        return measurements::run(directory, phase);
    }
    let mut store = Store::open(&directory.join("automerge-lifecycle.sqlite3"))?;
    match phase {
        "lifecycle-prepare" => {
            seed(&mut store)?;
            let mut a = store.read("A")?;
            let mut b = store.read("B")?;
            history::save_pair(&mut a.doc, "k0", "Позже", "Известная правка")?;
            history::save_pair(&mut b.doc, "k0", "Без связи", "Независимая правка")?;
            store.write("A", &a)?;
            store.write("B", &b)?;
        }
        "lifecycle-restore" => {
            let before = store.read("A")?.fingerprint()?;
            restore(&mut store, "initial", &before)?;
            assert!(paused(&store)?);
            assert!(sync(&mut store).is_err());
            assert_eq!(
                view(&store.read("A")?.doc)?["k0"].fields["title"][0].value,
                "Снимок"
            );
            read_snapshot(&store, "before-restore")?;
        }
        "lifecycle-resume" => {
            assert!(paused(&store)?);
            resume(&mut store)?;
            sync(&mut store)?;
            let nodes = view(&store.read("A")?.doc)?;
            assert!(conflicting(&nodes["k0"].fields["title"]));
            assert_eq!(nodes["k0"].fields["title"].len(), 2);
            assert_eq!(
                store.read("A")?.doc.get_heads(),
                store.read("B")?.doc.get_heads()
            );
            delete_snapshot(&mut store, "initial")?;
            read_snapshot(&store, "before-restore")?;
        }
        _ => return Err("Unknown lifecycle phase".into()),
    }
    println!("P02.06 {phase}: passed");
    Ok(())
}

mod maintenance;
pub(in super::super) mod measurements;
#[cfg(test)]
mod tests;
