use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::Connection;

// Core Data entity ids, read from Z_PRIMARYKEY (same convention as
// mc_export's ENT_* constants — stable per app version, hardcoded here).
const ENT_ALBUM: i64 = 10;
const ENT_DISC: i64 = 11;
const ENT_TRACK: i64 = 14;
const ENT_ARTIST: i64 = 3;
const ENT_ARTIST_PERSON: i64 = 4;
const ENT_CHORUS: i64 = 6;
const ENT_CHORUS_PERSON: i64 = 7;
const ENT_COMPOSER: i64 = 16;
const ENT_COMPOSER_PERSON: i64 = 17;
const ENT_CONDUCTOR: i64 = 21;
const ENT_CONDUCTOR_PERSON: i64 = 22;
const ENT_MUSICIAN: i64 = 40;
const ENT_MUSICIAN_PERSON: i64 = 41;
const ENT_ORCHESTRA: i64 = 42;
const ENT_ORCHESTRA_PERSON: i64 = 43;
const ENT_FORMAT: i64 = 27;
const ENT_GENRE: i64 = 28;
const ENT_TAG: i64 = 54;
const ENT_EXTRA: i64 = 26;
const ENT_STUDIO: i64 = 53;
const ENT_LABEL: i64 = 32;
const ENT_COUNTRY: i64 = 23;
const ENT_CONDITION: i64 = 20;
const ENT_PACKAGING: i64 = 45;
const ENT_VINYL_COLOR: i64 = 58;
const ENT_SPARS: i64 = 50;
const ENT_OWNER: i64 = 44;
const ENT_LOCATION: i64 = 38;
const ENT_LOOKUP_ITEM_DELEGATE: i64 = 39;
const ENT_PERSON_DELEGATE: i64 = 46;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args, Default)]
struct AlbumFields {
    #[arg(long)]
    format: Option<String>,
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    country: Option<String>,
    #[arg(long)]
    condition: Option<String>,
    #[arg(long)]
    packaging: Option<String>,
    #[arg(long = "vinyl-color")]
    vinyl_color: Option<String>,
    #[arg(long)]
    spars: Option<String>,
    #[arg(long)]
    owner: Option<String>,
    #[arg(long)]
    location: Option<String>,
    #[arg(long = "release-year")]
    release_year: Option<String>,
    #[arg(long)]
    rating: Option<i64>,
    #[arg(long)]
    notes: Option<String>,
    #[arg(long)]
    upc: Option<String>,
}

/// Artist/Composer/Conductor/Musician/Orchestra/Chorus each have their
/// own dedicated entity + person-delegate pair (see mc_export's
/// load_role_names, which reads the same shape). One repeatable flag
/// per role, same convention as --artist already had.
#[derive(clap::Args, Default)]
struct RoleInputs {
    #[arg(long = "artist")]
    artist: Vec<String>,
    #[arg(long = "composer")]
    composer: Vec<String>,
    #[arg(long = "conductor")]
    conductor: Vec<String>,
    #[arg(long = "musician")]
    musician: Vec<String>,
    #[arg(long = "orchestra")]
    orchestra: Vec<String>,
    #[arg(long = "chorus")]
    chorus: Vec<String>,
}

impl RoleInputs {
    fn pairs(&self) -> [(&'static str, &[String]); 6] {
        [
            ("artist", &self.artist),
            ("composer", &self.composer),
            ("conductor", &self.conductor),
            ("musician", &self.musician),
            ("orchestra", &self.orchestra),
            ("chorus", &self.chorus),
        ]
    }

    fn get(&self, key: &str) -> &[String] {
        self.pairs()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap_or(&[])
    }
}

/// Mirrors RoleInputs' keys — whether to replace (rather than leave
/// untouched) each role's list on an edit. Named in the plural, as
/// the CLI flags read more naturally that way (--replace-artists).
#[derive(clap::Args, Default)]
struct RoleReplaceFlags {
    #[arg(long)]
    replace_artists: bool,
    #[arg(long)]
    replace_composers: bool,
    #[arg(long)]
    replace_conductors: bool,
    #[arg(long)]
    replace_musicians: bool,
    #[arg(long)]
    replace_orchestras: bool,
    #[arg(long)]
    replace_choruses: bool,
}

impl RoleReplaceFlags {
    fn pairs(&self) -> [(&'static str, bool); 6] {
        [
            ("artist", self.replace_artists),
            ("composer", self.replace_composers),
            ("conductor", self.replace_conductors),
            ("musician", self.replace_musicians),
            ("orchestra", self.replace_orchestras),
            ("chorus", self.replace_choruses),
        ]
    }
}

struct RoleSpec {
    key: &'static str,
    role_table: &'static str,
    role_entity: &'static str,
    role_ent_id: i64,
    person_table: &'static str,
    person_entity: &'static str,
    person_ent_id: i64,
    back_ref: &'static str,
}

const ROLE_SPECS: &[RoleSpec] = &[
    RoleSpec {
        key: "artist",
        role_table: "ZARTIST",
        role_entity: "Artist",
        role_ent_id: ENT_ARTIST,
        person_table: "ZARTISTPERSON",
        person_entity: "ArtistPerson",
        person_ent_id: ENT_ARTIST_PERSON,
        back_ref: "ZARTISTPERSON",
    },
    RoleSpec {
        key: "composer",
        role_table: "ZCOMPOSER",
        role_entity: "Composer",
        role_ent_id: ENT_COMPOSER,
        person_table: "ZCOMPOSERPERSON",
        person_entity: "ComposerPerson",
        person_ent_id: ENT_COMPOSER_PERSON,
        back_ref: "ZCOMPOSERPERSON",
    },
    RoleSpec {
        key: "conductor",
        role_table: "ZCONDUCTOR",
        role_entity: "Conductor",
        role_ent_id: ENT_CONDUCTOR,
        person_table: "ZCONDUCTORPERSON",
        person_entity: "ConductorPerson",
        person_ent_id: ENT_CONDUCTOR_PERSON,
        back_ref: "ZCONDUCTORPERSON",
    },
    RoleSpec {
        key: "musician",
        role_table: "ZMUSICIAN",
        role_entity: "Musician",
        role_ent_id: ENT_MUSICIAN,
        person_table: "ZMUSICIANPERSON",
        person_entity: "MusicianPerson",
        person_ent_id: ENT_MUSICIAN_PERSON,
        back_ref: "ZMUSICIANPERSON",
    },
    RoleSpec {
        key: "orchestra",
        role_table: "ZORCHESTRA",
        role_entity: "Orchestra",
        role_ent_id: ENT_ORCHESTRA,
        person_table: "ZORCHESTRAPERSON",
        person_entity: "OrchestraPerson",
        person_ent_id: ENT_ORCHESTRA_PERSON,
        back_ref: "ZORCHESTRAPERSON",
    },
    RoleSpec {
        key: "chorus",
        role_table: "ZCHORUS",
        role_entity: "Chorus",
        role_ent_id: ENT_CHORUS,
        person_table: "ZCHORUSPERSON",
        person_entity: "ChorusPerson",
        person_ent_id: ENT_CHORUS_PERSON,
        back_ref: "ZCHORUSPERSON",
    },
];

fn role_spec(key: &str) -> &'static RoleSpec {
    ROLE_SPECS
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("clé de rôle inconnue: {key}"))
}

/// Genre/Tag/Extra/Studio are many-to-many via a Core Data join table
/// (e.g. Z_10GENRES) linking the album to a simple lookup entity — same
/// resolve-or-create as Format/Label/..., just through a join table
/// instead of a direct column on ZCOLLECTIBLE. Mirrors mc_export's
/// MANY_TO_MANY table.
#[derive(clap::Args, Default)]
struct ManyToManyInputs {
    #[arg(long = "genre")]
    genre: Vec<String>,
    #[arg(long = "tag")]
    tag: Vec<String>,
    #[arg(long = "extra")]
    extra: Vec<String>,
    #[arg(long = "studio")]
    studio: Vec<String>,
}

impl ManyToManyInputs {
    fn pairs(&self) -> [(&'static str, &[String]); 4] {
        [
            ("genre", &self.genre),
            ("tag", &self.tag),
            ("extra", &self.extra),
            ("studio", &self.studio),
        ]
    }

    fn get(&self, key: &str) -> &[String] {
        self.pairs()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap_or(&[])
    }
}

#[derive(clap::Args, Default)]
struct ManyToManyReplaceFlags {
    #[arg(long)]
    replace_genres: bool,
    #[arg(long)]
    replace_tags: bool,
    #[arg(long)]
    replace_extras: bool,
    #[arg(long)]
    replace_studios: bool,
}

impl ManyToManyReplaceFlags {
    fn pairs(&self) -> [(&'static str, bool); 4] {
        [
            ("genre", self.replace_genres),
            ("tag", self.replace_tags),
            ("extra", self.replace_extras),
            ("studio", self.replace_studios),
        ]
    }
}

struct ManyToManySpec {
    key: &'static str,
    join_table: &'static str,
    album_col: &'static str,
    target_col: &'static str,
    target_table: &'static str,
    target_entity: &'static str,
    target_ent_id: i64,
    back_ref: &'static str,
}

const MANY_TO_MANY_SPECS: &[ManyToManySpec] = &[
    ManyToManySpec {
        key: "genre",
        join_table: "Z_10GENRES",
        album_col: "Z_10ALBUMS2",
        target_col: "Z_28GENRES",
        target_table: "ZGENRE",
        target_entity: "Genre",
        target_ent_id: ENT_GENRE,
        back_ref: "ZGENRE",
    },
    ManyToManySpec {
        key: "tag",
        join_table: "Z_8TAGS",
        album_col: "Z_8COLLECTIBLES",
        target_col: "Z_54TAGS",
        target_table: "ZTAG",
        target_entity: "Tag",
        target_ent_id: ENT_TAG,
        back_ref: "ZTAG",
    },
    ManyToManySpec {
        key: "extra",
        join_table: "Z_10EXTRAS",
        album_col: "Z_10ALBUMS1",
        target_col: "Z_26EXTRAS",
        target_table: "ZEXTRA",
        target_entity: "Extra",
        target_ent_id: ENT_EXTRA,
        back_ref: "ZEXTRA",
    },
    ManyToManySpec {
        key: "studio",
        join_table: "Z_10STUDIOS",
        album_col: "Z_10ALBUMS4",
        target_col: "Z_53STUDIOS",
        target_table: "ZSTUDIO",
        target_entity: "Studio",
        target_ent_id: ENT_STUDIO,
        back_ref: "ZSTUDIO",
    },
];

fn many_to_many_spec(key: &str) -> &'static ManyToManySpec {
    MANY_TO_MANY_SPECS
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("clé many-to-many inconnue: {key}"))
}

#[derive(Subcommand)]
enum Command {
    /// Compute the SQL to insert a new album, without writing anything
    AddPlan {
        /// Path to a MyCollection.mucp package (never the live file — see the safety gate)
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        title: String,
        #[command(flatten)]
        roles: RoleInputs,
        #[command(flatten)]
        many: ManyToManyInputs,
        #[command(flatten)]
        fields: AlbumFields,
        /// One track title per occurrence, in order
        #[arg(long = "track")]
        tracks: Vec<String>,
        /// Write the plan's SQL to this file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the SQL to modify an existing album (by id), without writing anything
    EditPlan {
        #[arg(long)]
        db: PathBuf,
        /// The album's Z_PK (see mc_export's JSON output, field "id")
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[command(flatten)]
        fields: AlbumFields,
        #[command(flatten)]
        replace: RoleReplaceFlags,
        #[command(flatten)]
        roles: RoleInputs,
        #[command(flatten)]
        replace_many: ManyToManyReplaceFlags,
        #[command(flatten)]
        many: ManyToManyInputs,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the SQL to delete an album (by id) and everything specific
    /// to it (its artists/genres links, discs, tracks) — never the shared
    /// lookups (Format/Label/.../ArtistPerson), which other albums may
    /// still reference.
    DeletePlan {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        id: i64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Execute a plan file's SQL against the database, inside one transaction
    Commit {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        plan: PathBuf,
    },
}

/// Refuses to operate on what looks like a real, live Music Collector
/// collection — only ever a test/working copy. Same hard rule as the
/// BookNojo project's tools.
fn ensure_not_real_collection(path: &Path) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase();
    if text.contains("collectorz") && (text.contains("music collector") || text.contains("music_collector")) {
        bail!(
            "{} ressemble au chemin d'une vraie collection Music Collector \
             (~/Documents/Collectorz/...) — refusé. N'utiliser que des copies de test.",
            canonical.display()
        );
    }
    Ok(())
}

fn resolve_db_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("Music_data")
    } else {
        path.to_path_buf()
    }
}

/// Tracks Z_PRIMARYKEY.Z_MAX per entity across a whole plan (several new
/// rows of the same kind in one call must get distinct, sequential ids
/// before anything is actually written).
struct PkAllocator<'a> {
    conn: &'a Connection,
    cached_max: HashMap<String, i64>,
}

impl<'a> PkAllocator<'a> {
    fn new(conn: &'a Connection) -> Self {
        PkAllocator {
            conn,
            cached_max: HashMap::new(),
        }
    }

    fn next(&mut self, entity_name: &str) -> Result<i64> {
        if !self.cached_max.contains_key(entity_name) {
            let current: i64 = self.conn.query_row(
                "SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = ?1",
                [entity_name],
                |row| row.get(0),
            )?;
            self.cached_max.insert(entity_name.to_string(), current);
        }
        let next = self.cached_max[entity_name] + 1;
        self.cached_max.insert(entity_name.to_string(), next);
        Ok(next)
    }

    /// One UPDATE per touched entity, reflecting the final allocated max.
    fn bump_statements(&self) -> Vec<String> {
        self.cached_max
            .iter()
            .map(|(name, max)| {
                format!("UPDATE Z_PRIMARYKEY SET Z_MAX = {max} WHERE Z_NAME = '{name}';")
            })
            .collect()
    }
}

fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Finds an existing simple lookup (Format/Label/Country/...) by exact
/// display name, or plans the two inserts needed to create one
/// (the lookup entity row + its ZLOOKUPITEMDELEGATE row).
#[allow(clippy::too_many_arguments)]
fn resolve_or_create_simple_lookup(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    table: &str,
    entity_name: &str,
    ent_id: i64,
    back_ref_column: &str,
    name: &str,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT t.Z_PK FROM {table} t \
                 JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE \
                 WHERE l.ZDISPLAYNAME = ?1"
            ),
            [name],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_id = pks.next(entity_name)?;
    let new_delegate_id = pks.next("LookupItemDelegate")?;
    sql.push(format!(
        "INSERT INTO {table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE, ZITEMDELEGATE) \
         VALUES ({new_id}, {ent_id}, 1, {new_delegate_id}, NULL);"
    ));
    sql.push(format!(
        "INSERT INTO ZLOOKUPITEMDELEGATE (Z_PK, Z_ENT, Z_OPT, {back_ref_column}, ZDISPLAYNAME) \
         VALUES ({new_delegate_id}, {ent}, 1, {new_id}, {name_sql});",
        ent = ENT_LOOKUP_ITEM_DELEGATE,
        name_sql = sql_quote(name)
    ));
    Ok(new_id)
}

/// Finds an existing Artist/Composer/.../person by exact display name,
/// or plans the delegate + role-person rows needed to create one.
#[allow(clippy::too_many_arguments)]
fn resolve_or_create_person(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    person_table: &str,
    person_entity_name: &str,
    person_ent_id: i64,
    back_ref_column: &str,
    name: &str,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT pt.Z_PK FROM {person_table} pt \
                 JOIN ZPERSONDELEGATE p ON p.Z_PK = pt.ZDELEGATE \
                 WHERE p.ZDISPLAYNAME = ?1"
            ),
            [name],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_person_id = pks.next(person_entity_name)?;
    let new_delegate_id = pks.next("PersonDelegate")?;
    sql.push(format!(
        "INSERT INTO {person_table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE, ZITEMDELEGATE) \
         VALUES ({new_person_id}, {person_ent_id}, 1, {new_delegate_id}, NULL);"
    ));
    sql.push(format!(
        "INSERT INTO ZPERSONDELEGATE (Z_PK, Z_ENT, Z_OPT, {back_ref_column}, ZDISPLAYNAME, ZSORTNAME) \
         VALUES ({new_delegate_id}, {ent}, 1, {new_person_id}, {name_sql}, {name_sql});",
        ent = ENT_PERSON_DELEGATE,
        name_sql = sql_quote(name)
    ));
    Ok(new_person_id)
}

/// Resolves-or-creates each named person for one role, and plans the
/// role-link row (e.g. a ZARTIST row) for each, ranked in list order.
fn add_role_rows(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    spec: &RoleSpec,
    album_id: i64,
    names: &[String],
) -> Result<()> {
    for (rank, name) in names.iter().enumerate() {
        let person_id = resolve_or_create_person(
            conn,
            pks,
            sql,
            spec.person_table,
            spec.person_entity,
            spec.person_ent_id,
            spec.back_ref,
            name,
        )?;
        let role_id = pks.next(spec.role_entity)?;
        sql.push(format!(
            "INSERT INTO {role_table} (Z_PK, Z_ENT, Z_OPT, ZRANK, ZALBUM, {back_ref}) \
             VALUES ({role_id}, {role_ent_id}, 1, {rank}, {album_id}, {person_id});",
            role_table = spec.role_table,
            role_ent_id = spec.role_ent_id,
            back_ref = spec.back_ref,
        ));
    }
    Ok(())
}

/// Resolves-or-creates each named lookup for one many-to-many field
/// (Genre/Tag/Extra/Studio), and plans the join-table row for each.
fn add_many_to_many_rows(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    spec: &ManyToManySpec,
    album_id: i64,
    names: &[String],
) -> Result<()> {
    for name in names {
        let target_id = resolve_or_create_simple_lookup(
            conn,
            pks,
            sql,
            spec.target_table,
            spec.target_entity,
            spec.target_ent_id,
            spec.back_ref,
            name,
        )?;
        sql.push(format!(
            "INSERT INTO {join_table} ({album_col}, {target_col}) VALUES ({album_id}, {target_id});",
            join_table = spec.join_table,
            album_col = spec.album_col,
            target_col = spec.target_col,
        ));
    }
    Ok(())
}

// (field_name_in_struct_access, column, entity_name, ent_id, back_ref_column)
// handled via a small helper below instead of a table, since each lookup
// has a differently-named accessor on AlbumFields.
#[allow(clippy::too_many_arguments)]
fn resolve_simple_fields(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    fields: &AlbumFields,
) -> Result<HashMap<&'static str, i64>> {
    let mut ids = HashMap::new();
    macro_rules! resolve {
        ($field:expr, $key:literal, $table:literal, $entity:literal, $ent_id:expr, $back_ref:literal) => {
            if let Some(name) = $field.as_deref() {
                let id = resolve_or_create_simple_lookup(
                    conn, pks, sql, $table, $entity, $ent_id, $back_ref, name,
                )?;
                ids.insert($key, id);
            }
        };
    }
    resolve!(fields.format, "format", "ZFORMAT", "Format", ENT_FORMAT, "ZFORMAT");
    resolve!(fields.label, "label", "ZLABEL", "Label", ENT_LABEL, "ZLABEL");
    resolve!(fields.country, "country", "ZCOUNTRY", "Country", ENT_COUNTRY, "ZCOUNTRY");
    resolve!(fields.condition, "condition", "ZCONDITION", "Condition", ENT_CONDITION, "ZCONDITION");
    resolve!(fields.packaging, "packaging", "ZPACKAGING", "Packaging", ENT_PACKAGING, "ZPACKAGING");
    resolve!(fields.vinyl_color, "vinyl_color", "ZVINYLCOLOR", "VinylColor", ENT_VINYL_COLOR, "ZVINYLCOLOR");
    resolve!(fields.spars, "spars", "ZSPARS", "Spars", ENT_SPARS, "ZSPARS");
    resolve!(fields.owner, "owner", "ZOWNER", "Owner", ENT_OWNER, "ZOWNER");
    resolve!(fields.location, "location", "ZLOCATION", "Location", ENT_LOCATION, "ZLOCATION");
    Ok(ids)
}

fn scalar_text_assignments(fields: &AlbumFields) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if let Some(v) = &fields.release_year {
        out.push(("ZRELEASEYEAR", sql_quote(v)));
    }
    if let Some(v) = fields.rating {
        out.push(("ZMYRATING", v.to_string()));
    }
    if let Some(v) = &fields.notes {
        out.push(("ZNOTES", sql_quote(v)));
    }
    if let Some(v) = &fields.upc {
        out.push(("ZUPC", sql_quote(v)));
    }
    out
}

fn build_plan(
    conn: &Connection,
    title: &str,
    roles: &RoleInputs,
    many: &ManyToManyInputs,
    fields: &AlbumFields,
    tracks: &[String],
) -> Result<String> {
    let mut pks = PkAllocator::new(conn);
    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];

    let lookup_ids = resolve_simple_fields(conn, &mut pks, &mut sql, fields)?;

    let album_id = pks.next("Collectible")?;
    let mut columns = vec![
        "Z_PK".to_string(),
        "Z_ENT".to_string(),
        "Z_OPT".to_string(),
        "ZCOLLECTIONSTATUS".to_string(),
        "ZNRTRACKS".to_string(),
        "ZTITLE".to_string(),
    ];
    let mut values = vec![
        album_id.to_string(),
        ENT_ALBUM.to_string(),
        "1".to_string(),
        "2".to_string(),
        tracks.len().to_string(),
        sql_quote(title),
    ];
    for (key, column) in [
        ("format", "ZFORMAT"),
        ("label", "ZLABEL"),
        ("country", "ZCOUNTRY"),
        ("condition", "ZCONDITION"),
        ("packaging", "ZPACKAGING"),
        ("vinyl_color", "ZVINYLCOLOR"),
        ("spars", "ZSPARS"),
        ("owner", "ZOWNER"),
        ("location", "ZLOCATION"),
    ] {
        if let Some(id) = lookup_ids.get(key) {
            columns.push(column.to_string());
            values.push(id.to_string());
        }
    }
    for (column, value) in scalar_text_assignments(fields) {
        columns.push(column.to_string());
        values.push(value);
    }
    sql.push(format!(
        "INSERT INTO ZCOLLECTIBLE ({}) VALUES ({});",
        columns.join(", "),
        values.join(", ")
    ));

    for (key, names) in roles.pairs() {
        if !names.is_empty() {
            add_role_rows(conn, &mut pks, &mut sql, role_spec(key), album_id, names)?;
        }
    }

    for (key, names) in many.pairs() {
        if !names.is_empty() {
            add_many_to_many_rows(
                conn,
                &mut pks,
                &mut sql,
                many_to_many_spec(key),
                album_id,
                names,
            )?;
        }
    }

    if !tracks.is_empty() {
        let disc_id = pks.next("Collectible")?;
        sql.push(format!(
            "INSERT INTO ZCOLLECTIBLE (Z_PK, Z_ENT, Z_OPT, ZALBUM) \
             VALUES ({disc_id}, {ENT_DISC}, 1, {album_id});"
        ));
        for (position, track_title) in tracks.iter().enumerate() {
            let track_id = pks.next("Collectible")?;
            sql.push(format!(
                "INSERT INTO ZCOLLECTIBLE (Z_PK, Z_ENT, Z_OPT, ZDISC1, ZPOSITION, ZTITLE) \
                 VALUES ({track_id}, {ENT_TRACK}, 1, {disc_id}, {position}, {title_sql});",
                position = position + 1,
                title_sql = sql_quote(track_title),
            ));
        }
    }

    sql.extend(pks.bump_statements());
    sql.push("COMMIT;".to_string());
    Ok(sql.join("\n"))
}

#[allow(clippy::too_many_arguments)]
fn build_edit_plan(
    conn: &Connection,
    album_id: i64,
    title: Option<&str>,
    fields: &AlbumFields,
    replace: &RoleReplaceFlags,
    roles: &RoleInputs,
    replace_many: &ManyToManyReplaceFlags,
    many: &ManyToManyInputs,
) -> Result<String> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
        [album_id, ENT_ALBUM],
        |row| row.get(0),
    )?;
    if exists == 0 {
        bail!("Aucun album avec l'id {album_id}");
    }

    let mut pks = PkAllocator::new(conn);
    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];

    let lookup_ids = resolve_simple_fields(conn, &mut pks, &mut sql, fields)?;
    let mut assignments: Vec<String> = Vec::new();
    if let Some(t) = title {
        assignments.push(format!("ZTITLE = {}", sql_quote(t)));
    }
    for (key, column) in [
        ("format", "ZFORMAT"),
        ("label", "ZLABEL"),
        ("country", "ZCOUNTRY"),
        ("condition", "ZCONDITION"),
        ("packaging", "ZPACKAGING"),
        ("vinyl_color", "ZVINYLCOLOR"),
        ("spars", "ZSPARS"),
        ("owner", "ZOWNER"),
        ("location", "ZLOCATION"),
    ] {
        if let Some(id) = lookup_ids.get(key) {
            assignments.push(format!("{column} = {id}"));
        }
    }
    for (column, value) in scalar_text_assignments(fields) {
        assignments.push(format!("{column} = {value}"));
    }
    if !assignments.is_empty() {
        sql.push(format!(
            "UPDATE ZCOLLECTIBLE SET {} WHERE Z_PK = {album_id};",
            assignments.join(", ")
        ));
    }

    for (key, do_replace) in replace.pairs() {
        if do_replace {
            let spec = role_spec(key);
            sql.push(format!(
                "DELETE FROM {} WHERE ZALBUM = {album_id};",
                spec.role_table
            ));
            add_role_rows(conn, &mut pks, &mut sql, spec, album_id, roles.get(key))?;
        }
    }

    for (key, do_replace) in replace_many.pairs() {
        if do_replace {
            let spec = many_to_many_spec(key);
            sql.push(format!(
                "DELETE FROM {} WHERE {} = {album_id};",
                spec.join_table, spec.album_col
            ));
            add_many_to_many_rows(conn, &mut pks, &mut sql, spec, album_id, many.get(key))?;
        }
    }

    sql.extend(pks.bump_statements());
    sql.push("COMMIT;".to_string());
    Ok(sql.join("\n"))
}

/// Deletes an album and everything specific to it — discs, tracks, and
/// its role/many-to-many link rows — but never the shared lookups
/// (Format/Label/.../ArtistPerson/PersonDelegate), since other albums
/// may still reference them. Leaving an unreferenced lookup row behind
/// is harmless; deleting one still referenced elsewhere is not.
fn build_delete_plan(conn: &Connection, album_id: i64) -> Result<String> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
        [album_id, ENT_ALBUM],
        |row| row.get(0),
    )?;
    if exists == 0 {
        bail!("Aucun album avec l'id {album_id}");
    }

    let disc_ids: Vec<i64> = conn
        .prepare("SELECT Z_PK FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 AND ZALBUM = ?2")?
        .query_map([ENT_DISC, album_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    let disc_list = disc_ids
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];
    for role_table in [
        "ZARTIST", "ZCOMPOSER", "ZCONDUCTOR", "ZMUSICIAN", "ZORCHESTRA", "ZCHORUS", "ZCREDIT",
    ] {
        sql.push(format!("DELETE FROM {role_table} WHERE ZALBUM = {album_id};"));
    }
    sql.push(format!(
        "DELETE FROM Z_10GENRES WHERE Z_10ALBUMS2 = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_8TAGS WHERE Z_8COLLECTIBLES = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_10EXTRAS WHERE Z_10ALBUMS1 = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_10STUDIOS WHERE Z_10ALBUMS4 = {album_id};"
    ));
    if !disc_ids.is_empty() {
        sql.push(format!(
            "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_TRACK} AND ZDISC1 IN ({disc_list});"
        ));
        sql.push(format!(
            "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_DISC} AND Z_PK IN ({disc_list});"
        ));
    }
    sql.push(format!(
        "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_ALBUM} AND Z_PK = {album_id};"
    ));
    sql.push("COMMIT;".to_string());
    Ok(sql.join("\n"))
}

fn open_readonly(db: &Path) -> Result<(PathBuf, Connection)> {
    ensure_not_real_collection(db)?;
    let db_path = resolve_db_path(db);
    let conn = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("ouverture de {}", db_path.display()))?;
    Ok((db_path, conn))
}

fn emit_plan(plan_sql: &str, out: Option<PathBuf>, db_path: &Path) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(&path, plan_sql)
                .with_context(|| format!("écriture de {}", path.display()))?;
            println!("Plan écrit dans: {}", path.display());
        }
        None => println!("{plan_sql}"),
    }
    println!(
        "(Aucune écriture dans {} — ce SQL n'a pas été exécuté.)",
        db_path.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::AddPlan {
            db,
            title,
            roles,
            many,
            fields,
            tracks,
            out,
        } => {
            let (db_path, conn) = open_readonly(&db)?;
            let plan_sql = build_plan(&conn, &title, &roles, &many, &fields, &tracks)?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::EditPlan {
            db,
            id,
            title,
            fields,
            replace,
            roles,
            replace_many,
            many,
            out,
        } => {
            let (db_path, conn) = open_readonly(&db)?;
            let plan_sql = build_edit_plan(
                &conn,
                id,
                title.as_deref(),
                &fields,
                &replace,
                &roles,
                &replace_many,
                &many,
            )?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::DeletePlan { db, id, out } => {
            let (db_path, conn) = open_readonly(&db)?;
            let plan_sql = build_delete_plan(&conn, id)?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::Commit { db, plan } => {
            ensure_not_real_collection(&db)?;
            let db_path = resolve_db_path(&db);
            let plan_sql = std::fs::read_to_string(&plan)
                .with_context(|| format!("lecture de {}", plan.display()))?;

            let conn = Connection::open(&db_path)
                .with_context(|| format!("ouverture de {}", db_path.display()))?;
            // plan_sql already wraps its statements in BEGIN;/COMMIT;
            conn.execute_batch(&plan_sql)
                .with_context(|| "exécution du plan")?;
            println!("Plan exécuté dans: {}", db_path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_writable(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        for entry in walk(path)? {
            let mode = if entry.is_dir() { 0o755 } else { 0o644 };
            std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(mode))?;
        }
        Ok(())
    }

    fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut out = vec![dir.to_path_buf()];
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.extend(walk(&entry.path())?);
            } else {
                out.push(entry.path());
            }
        }
        Ok(out)
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dst_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &dst_path)?;
            } else {
                std::fs::copy(entry.path(), &dst_path)?;
            }
        }
        Ok(())
    }

    #[test]
    fn add_edit_and_delete_round_trip() {
        let source = Path::new("testdata/MyCollection.mucp");
        if !source.exists() {
            eprintln!(
                "testdata/MyCollection.mucp absent (copie personnelle, non committée) — test ignoré"
            );
            return;
        }

        let scratch = std::env::temp_dir().join(format!(
            "mc_add_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        copy_dir(source, &scratch).unwrap();
        // testdata/ is read-only (chmod 444/555); std::fs::copy preserves
        // that, so make this disposable scratch copy writable again.
        make_writable(&scratch).unwrap();
        // chmod/ownership issues aside, a disposable scratch copy is never
        // the shared testdata/ or testdata-rw/ fixtures, nor the real file.
        ensure_not_real_collection(&scratch).unwrap();

        let db_path = scratch.join("Music_data");
        let read_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let fields = AlbumFields {
            format: Some("CD".to_string()),
            ..Default::default()
        };
        let roles = RoleInputs {
            artist: vec!["Artiste de test".to_string()],
            composer: vec!["Compositeur de test".to_string()],
            ..Default::default()
        };
        let many = ManyToManyInputs {
            genre: vec!["Genre de test".to_string()],
            tag: vec!["Tag de test".to_string()],
            ..Default::default()
        };
        let plan_sql = build_plan(
            &read_conn,
            "Test unitaire mc_add",
            &roles,
            &many,
            &fields,
            &["Piste unique".to_string()],
        )
        .unwrap();
        drop(read_conn);

        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&plan_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let (album_id, title): (i64, String) = verify_conn
            .query_row(
                "SELECT Z_PK, ZTITLE FROM ZCOLLECTIBLE WHERE ZTITLE = 'Test unitaire mc_add'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(title, "Test unitaire mc_add");
        let composer_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM ZCOMPOSER WHERE ZALBUM = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(composer_count, 1);
        let tag_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_8TAGS WHERE Z_8COLLECTIBLES = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count, 1);

        // Edit: rename, replace the genre/composer/tag lists.
        let edit_sql = build_edit_plan(
            &verify_conn,
            album_id,
            Some("Titre modifié"),
            &AlbumFields::default(),
            &RoleReplaceFlags {
                replace_composers: true,
                ..Default::default()
            },
            &RoleInputs {
                composer: vec!["Autre compositeur".to_string()],
                ..Default::default()
            },
            &ManyToManyReplaceFlags {
                replace_genres: true,
                replace_tags: true,
                ..Default::default()
            },
            &ManyToManyInputs {
                genre: vec!["Autre genre".to_string()],
                tag: vec![],
                ..Default::default()
            },
        )
        .unwrap();
        drop(verify_conn);
        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&edit_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let renamed: String = verify_conn
            .query_row(
                "SELECT ZTITLE FROM ZCOLLECTIBLE WHERE Z_PK = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(renamed, "Titre modifié");
        let genre_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_10GENRES WHERE Z_10ALBUMS2 = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(genre_count, 1);
        let tag_count_after: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_8TAGS WHERE Z_8COLLECTIBLES = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count_after, 0);
        let new_composer_name: String = verify_conn
            .query_row(
                "SELECT p.ZDISPLAYNAME FROM ZCOMPOSER c \
                 JOIN ZCOMPOSERPERSON cp ON cp.Z_PK = c.ZCOMPOSERPERSON \
                 JOIN ZPERSONDELEGATE p ON p.Z_PK = cp.ZDELEGATE \
                 WHERE c.ZALBUM = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(new_composer_name, "Autre compositeur");

        // Delete: the album and its track should both disappear.
        let delete_sql = build_delete_plan(&verify_conn, album_id).unwrap();
        drop(verify_conn);
        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&delete_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let remaining: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);

        let _ = std::fs::remove_dir_all(&scratch);
    }
}
