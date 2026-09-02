use serde::Deserialize;
use std::{
    borrow::Cow,
    fmt::Display,
    sync::{Arc, OnceLock},
    time::Duration,
};
use turso::{Builder, Connection, Database, Row};
use twilight_model::http::attachment::Attachment;

use crate::AppState;

pub struct EditRequest {
    pub input: String,
    pub index: String,
}

pub struct PlanetResult {
    pub planet: Planet,
    pub result: Row,
    prettier: bool,
}

impl Display for PlanetResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let planet = &self.planet;

        let mut include_space = false;

        if self.prettier {
            write!(f, "```")?;
        } else {
            write!(f, "#-----------------------------------------#\n")?;
        }

        write!(
            f,
            "ID: {}\nStar Id: {}\nName: {}\nRadius: {} studs\nGravity: {:.2}g\nConditions: {}\nTemperature: {}C",
            &planet.id,
            &planet.star_id,
            &planet.name,
            &planet.radius,
            &planet.gravity,
            &planet.conditions,
            &planet.temperature,
        )?;
        if let Some(sector) = &planet.sector {
            write!(f, "\nSector: {}", sector)?;
        }
        write!(f, "\nTectonics: {}", &planet.tectonics)?;

        if let Some(atmosphere) = &planet.atmosphere {
            write!(f, "\nAtmosphere: {}", atmosphere)?;
        }
        if let Some(oceans) = &planet.oceans {
            write!(f, "\nOceans: {}", oceans)?;
        }
        if let Some(rings) = &planet.rings {
            write!(f, "\nRings: {}", rings)?;
        }
        if let Some(trees) = &planet.trees {
            write!(f, "\nTrees: {}", trees)?;
        }
        if let Some(sub_trees) = &planet.sub_trees {
            write!(f, "\nSub trees: {}", sub_trees)?;
        }
        write!(f, "\nLife: {}", planet.life)?;
        if let Some(life_type) = &planet.life_type {
            write!(f, "\nLife type: {}", life_type)?;
        }
        write!(f, "\nIs Moon: {}", planet.is_moon)?;
        if let Some(moons) = &planet.moons {
            write!(f, "\nMoons: {}", moons)?;
        }
        writeln!(f)?;

        if let Some(malachite) = &planet.malachite {
            write!(f, "\nMalachite: {}", malachite)?;
            include_space = true;
        }
        if let Some(hematite) = &planet.hematite {
            write!(f, "\nHematite: {:.4}", hematite)?;
            include_space = true;
        }
        if let Some(petroleum) = &planet.petroleum {
            write!(f, "\nPetroleum: {}", petroleum)?;
            include_space = true;
        }
        if let Some(coal) = &planet.coal {
            write!(f, "\nCoal: {}", coal)?;
            include_space = true;
        }
        if let Some(gummite) = &planet.gummite {
            write!(f, "\nGummite: {}", gummite)?;
            include_space = true;
        }
        if let Some(tektite) = &planet.tektite {
            write!(f, "\nTektite: {}", tektite)?;
            include_space = true;
        }
        if let Some(bauxite) = &planet.bauxite {
            write!(f, "\nBauxite: {}", bauxite)?;
            include_space = true;
        }
        if let Some(gold) = &planet.gold {
            write!(f, "\nGold: {:.4}", gold)?;
            include_space = true;
        }
        if let Some(cerussite) = &planet.cerussite {
            write!(f, "\nCerussite: {}", cerussite)?;
            include_space = true;
        }
        if include_space {
            writeln!(f)?;
            include_space = false;
        }

        if let Some(lime) = &planet.lime {
            write!(f, "\nLime: {}", lime)?;
            include_space = true;
        }
        if let Some(saltpeter) = &planet.saltpeter {
            write!(f, "\nSaltpeter: {}", saltpeter)?;
            include_space = true;
        }
        if let Some(quartz) = &planet.quartz {
            write!(f, "\nQuartz: {}", quartz)?;
            include_space = true;
        }
        if let Some(ice) = &planet.ice {
            write!(f, "\nIce: {}", ice)?;
            include_space = true;
        }

        if include_space {
            writeln!(f)?;
            include_space = false;
        }

        if let Some(note) = &planet.note {
            write!(f, "\nNote: {}", note)?;
        }

        if self.prettier {
            write!(f, "\n```")?;
        } else {
            write!(f, "\n#-----------------------------------------#")?;
        }

        if include_space {
            writeln!(f)?;
        }
        Ok(())
    }
}
struct DatabaseStruct {
    database: Arc<Database>,
}

impl DatabaseStruct {
    async fn get_conn(&self) -> anyhow::Result<Connection> {
        let db_ref = Arc::clone(&self.database);

        let conn = db_ref.as_ref().connect()?;
        conn.busy_timeout(Duration::from_millis(500))?; // 0.5 seconds
        conn.pragma_update("journal_mode", "'mvcc'").await?; // enables concurrency writes which is good!

        Ok(conn)
    }
}

static DB: OnceLock<DatabaseStruct> = OnceLock::new();
static CHECKS: OnceLock<Checks> = OnceLock::new();

#[derive(Deserialize)]
struct Checks {
    #[serde(rename = "allowed_sectors")]
    sectors: Vec<String>,
    #[serde(rename = "allowed_tectonics")]
    tectonics: Vec<String>,
    #[serde(rename = "allowed_atmospheres")]
    atmospheres: Vec<String>,
    #[serde(skip)]
    #[serde(rename = "allowed_oceans")]
    #[allow(dead_code)]
    oceans: Vec<String>,
    #[serde(skip)]
    #[serde(rename = "allowed_trees")]
    #[allow(dead_code)]
    trees: Vec<String>, // won't be used
}

#[derive(sea_query::Iden)]
enum Planets {
    Table,
    Id,
    StarId,
    Name,
    Radius,
    Gravity,
    Conditions,
    Temperature,
    Sector,
    Tectonics,
    Atmosphere,
    Oceans,
    Rings,
    Trees,
    SubTrees,
    Life,
    LifeType,
    IsMoon,
    Moons,
    Malachite,
    Hematite,
    Petroleum,
    Coal,
    Gummite,
    Tektite,
    Bauxite,
    Gold,
    Cerussite,
    Lime,
    Saltpeter,
    Quartz,
    Ice,
    Note,
}

#[allow(dead_code)] // yes
#[derive(Deserialize)]
pub struct Planet {
    id: String,
    star_id: i64,
    // default: "Unknown"
    name: String,
    // default: 0 for radius, gravity and temp
    radius: f64,
    gravity: f64,
    // default: "Unknown"
    conditions: String,
    temperature: i64,
    sector: Option<String>,
    // Default: "Unknown" for tectonics
    tectonics: String,
    atmosphere: Option<String>,
    oceans: Option<String>,
    rings: Option<String>,
    trees: Option<String>,
    sub_trees: Option<String>,
    // default: false (only for life)
    life: bool,
    life_type: Option<String>,
    is_moon: bool,
    moons: Option<i32>,

    malachite: Option<i32>,
    hematite: Option<f64>,
    petroleum: Option<i32>,
    coal: Option<i32>,
    gummite: Option<i32>,
    tektite: Option<i32>,
    bauxite: Option<i32>,
    gold: Option<f64>,
    cerussite: Option<i32>,

    lime: Option<bool>,
    saltpeter: Option<bool>,
    quartz: Option<bool>,
    ice: Option<bool>,

    note: Option<String>,
}

pub async fn establish_database(database_url: &str) -> anyhow::Result<()> {
    let db = Builder::new_local(database_url).build().await?;
    DB.set(DatabaseStruct {
        database: Arc::new(db),
    })
    .map_err(|_| anyhow::anyhow!("DB is already initialized"))?;

    let check: Checks = {
        let content = std::fs::read("configs.json")?;
        serde_json::from_slice(&content)?
    };
    CHECKS
        .set(check)
        .map_err(|_| anyhow::anyhow!("CHECKS is already initialized"))?;

    Ok(())
}

pub async fn get_planet(index: &str, state: AppState) -> anyhow::Result<PlanetResult> {
    if check_sql(index, state) {
        anyhow::bail!("Blacklisted sql");
    }
    check_index(index)?;

    let conn = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?
        .get_conn()
        .await?;

    let mut planets = conn
        .query("SELECT * FROM planets WHERE id = ?1", [index.to_string()])
        .await?;

    if let Some(row) = planets.next().await? {
        return Ok(PlanetResult {
            planet: construct_planet(&row)?,
            result: row,
            prettier: true,
        });
    }

    Err(anyhow::anyhow!("Planet wasn't found"))
}

pub async fn remove_planet(index: &str, state: AppState) -> anyhow::Result<()> {
    if check_sql(index, state) {
        anyhow::bail!("Blacklisted sql");
    }

    check_index(index)?;

    let conn = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?
        .get_conn()
        .await?;

    conn.execute("DELETE FROM planets WHERE id = ?1", (index.to_string(),))
        .await?;

    Ok(())
}

// gon be reworked
pub async fn search_planets(input: &str, state: AppState) -> anyhow::Result<Attachment> {
    if check_sql(input, state) {
        anyhow::bail!("Blacklisted sql");
    }

    let conn = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?
        .get_conn()
        .await?;
    let input = normalize(&input);

    let mut planets = conn
        .query(format!("SELECT * FROM planets WHERE {}", input), ())
        .await?;

    let results_limit = 100;
    let mut results_showed = 0;
    let mut file_content = format!("Only showing first {results_limit} results\n");

    while let Some(row) = planets.next().await? {
        if results_showed >= results_limit {
            break;
        }

        let result = PlanetResult {
            planet: construct_planet(&row)?,
            result: row,
            prettier: false,
        };

        file_content.push_str(&format!("\n{}", result.to_string()));

        results_showed += 1;
    }

    Ok(Attachment::from_bytes(
        "result.txt".to_owned(),
        file_content.into_bytes(),
        0,
    ))
}

pub async fn edit_planet(query: &EditRequest, bypass: bool) -> anyhow::Result<()> {
    let index = &query.index;
    let input = &query.input;
    let conn = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("DB is not initialized"))?
        .get_conn()
        .await?;

    let input = normalize(&input);

    let mut split_iter = index.split('-');
    let star_id: i64 = split_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid star id format"))?
        .parse()?;

    if split_iter.next().unwrap_or("None").parse::<i64>().is_err() {
        anyhow::bail!("Invalid planet id format");
    }

    let mut is_moon = match split_iter.next() {
        Some(s) => {
            s.parse::<i64>()?;
            true
        }
        None => false,
    };

    if split_iter.next().is_some() {
        anyhow::bail!("Invalid id format");
    }

    let mut name: Option<String> = None;
    let mut radius: Option<f64> = None;
    let mut gravity: Option<f64> = None;
    let mut conditions: Option<String> = None;
    let mut temperature: Option<i64> = None;
    let mut sector: Option<String> = None;
    let mut tectonics: Option<String> = None;
    let mut atmosphere: Option<String> = None;
    let mut oceans: Option<String> = None;
    let mut rings: Option<String> = None;
    let mut trees: Option<String> = None;
    let mut sub_trees: Option<String> = None;
    let mut life: Option<bool> = None;
    let mut life_type: Option<String> = None;
    let mut moons: Option<i32> = None;

    let mut malachite: Option<i32> = None;
    let mut hematite: Option<f64> = None;
    let mut petroleum: Option<i32> = None;
    let mut coal: Option<i32> = None;
    let mut gummite: Option<i32> = None;
    let mut tektite: Option<i32> = None;
    let mut bauxite: Option<i32> = None;
    let mut gold: Option<f64> = None;
    let mut cerussite: Option<i32> = None;

    let mut lime: Option<bool> = None;
    let mut saltpeter: Option<bool> = None;
    let mut quartz: Option<bool> = None;
    let mut ice: Option<bool> = None;

    let mut note: Option<String> = None;

    for expr in input.split("|") {
        let expr = expr.trim();
        if expr.is_empty() {
            continue;
        }

        let (key, mut value) = expr
            .split_once("=")
            .ok_or(anyhow::anyhow!("Invalid input"))?;
        let key = key.trim().to_lowercase();
        value = value.trim();

        validate(&key, &value)?;
        match key.as_str() {
            "name" => name = Some(value.to_string()),
            "radius" => radius = Some(value.parse()?),
            "gravity" => gravity = Some(value.parse()?),
            "conditions" => conditions = Some(value.to_string()),
            "temperature" => temperature = Some(value.parse()?),
            "sector" => sector = Some(value.to_string()),
            "tectonics" => tectonics = Some(value.to_string()),
            "atmosphere" => atmosphere = Some(value.to_string()),
            "oceans" => oceans = Some(value.to_string()),
            "rings" => rings = Some(value.to_string()),
            "trees" => trees = Some(value.to_string()),
            "sub trees" => sub_trees = Some(value.to_string()),
            "life" => life = Some(value.parse::<bool>()?),
            "life type" => life_type = Some(value.to_string()),
            "moons" => moons = Some(value.parse()?),
            "malachite" => malachite = Some(value.parse()?),
            "hematite" => hematite = Some((value.parse::<f64>()? * 1000.0).round() / 1000.0),
            "petroleum" => petroleum = Some(value.parse()?),
            "coal" => coal = Some(value.parse()?),
            "gummite" => gummite = Some(value.parse()?),
            "tektite" => tektite = Some(value.parse()?),
            "bauxite" => bauxite = Some(value.parse()?),
            "gold" => gold = Some((value.parse::<f64>()? * 1000.0).round() / 1000.0),
            "cerussite" => cerussite = Some(value.parse()?),
            "lime" => lime = Some(value.parse()?),
            "saltpeter" => saltpeter = Some(value.parse()?),
            "quartz" => quartz = Some(value.parse()?),
            "ice" => ice = Some(value.parse()?),
            "note" => note = Some(value.to_string()),
            "moon" if bypass => is_moon = value.parse::<bool>()?,
            _ => continue,
        }
    }

    let (sql, query_values) = {
        let mut columns = vec![Planets::Id, Planets::StarId];
        let mut values: Vec<sea_query::SimpleExpr> = vec![index.into(), star_id.into()];

        let mut conflict = sea_query::OnConflict::new();

        if let Some(name) = name {
            columns.push(Planets::Name);
            values.push(name.into());
            conflict.update_column(Planets::Name);
        }
        if let Some(radius) = radius {
            columns.push(Planets::Radius);
            values.push(radius.into());
            conflict.update_column(Planets::Radius);
        }
        if let Some(gravity) = gravity {
            columns.push(Planets::Gravity);
            values.push(gravity.into());
            conflict.update_column(Planets::Gravity);
        }
        if let Some(conditions) = conditions {
            columns.push(Planets::Conditions);
            values.push(conditions.into());
            conflict.update_column(Planets::Conditions);
        }
        if let Some(temperature) = temperature {
            columns.push(Planets::Temperature);
            values.push(temperature.into());
            conflict.update_column(Planets::Temperature);
        }
        if let Some(tectonics) = tectonics {
            columns.push(Planets::Tectonics);
            values.push(tectonics.into());
            conflict.update_column(Planets::Tectonics);
        }
        if let Some(sector) = sector {
            columns.push(Planets::Sector);
            values.push(sector.into());
            conflict.update_column(Planets::Sector);
        }
        if let Some(atmosphere) = atmosphere {
            columns.push(Planets::Atmosphere);
            values.push(atmosphere.into());
            conflict.update_column(Planets::Atmosphere);
        }
        if let Some(oceans) = oceans {
            columns.push(Planets::Oceans);
            values.push(oceans.into());
            conflict.update_column(Planets::Oceans);
        }
        if let Some(rings) = rings {
            columns.push(Planets::Rings);
            values.push(rings.into());
            conflict.update_column(Planets::Rings);
        }
        if let Some(trees) = trees {
            columns.push(Planets::Trees);
            values.push(trees.into());
            conflict.update_column(Planets::Trees);
        }
        if let Some(sub_trees) = sub_trees {
            columns.push(Planets::SubTrees);
            values.push(sub_trees.into());
            conflict.update_column(Planets::SubTrees);
        }
        if let Some(life) = life {
            columns.push(Planets::Life);
            values.push(life.into());
            conflict.update_column(Planets::Life);
        }
        if let Some(life_type) = life_type {
            columns.push(Planets::LifeType);
            values.push(life_type.into());
            conflict.update_column(Planets::LifeType);
        }
        if let Some(moons) = moons {
            columns.push(Planets::Moons);
            values.push(moons.into());
            conflict.update_column(Planets::Moons);
        }
        if let Some(malachite) = malachite {
            columns.push(Planets::Malachite);
            values.push(malachite.into());
            conflict.update_column(Planets::Malachite);
        }
        if let Some(hematite) = hematite {
            columns.push(Planets::Hematite);
            values.push(hematite.into());
            conflict.update_column(Planets::Hematite);
        }
        if let Some(petroleum) = petroleum {
            columns.push(Planets::Petroleum);
            values.push(petroleum.into());
            conflict.update_column(Planets::Petroleum);
        }
        if let Some(coal) = coal {
            columns.push(Planets::Coal);
            values.push(coal.into());
            conflict.update_column(Planets::Coal);
        }
        if let Some(gummite) = gummite {
            columns.push(Planets::Gummite);
            values.push(gummite.into());
            conflict.update_column(Planets::Gummite);
        }
        if let Some(tektite) = tektite {
            columns.push(Planets::Tektite);
            values.push(tektite.into());
            conflict.update_column(Planets::Tektite);
        }
        if let Some(bauxite) = bauxite {
            columns.push(Planets::Bauxite);
            values.push(bauxite.into());
            conflict.update_column(Planets::Bauxite);
        }
        if let Some(gold) = gold {
            columns.push(Planets::Gold);
            values.push(gold.into());
            conflict.update_column(Planets::Gold);
        }
        if let Some(cerussite) = cerussite {
            columns.push(Planets::Cerussite);
            values.push(cerussite.into());
            conflict.update_column(Planets::Cerussite);
        }

        if let Some(lime) = lime {
            columns.push(Planets::Lime);
            values.push(lime.into());
            conflict.update_column(Planets::Lime);
        }
        if let Some(saltpeter) = saltpeter {
            columns.push(Planets::Saltpeter);
            values.push(saltpeter.into());
            conflict.update_column(Planets::Saltpeter);
        }
        if let Some(quartz) = quartz {
            columns.push(Planets::Quartz);
            values.push(quartz.into());
            conflict.update_column(Planets::Quartz);
        }
        if let Some(ice) = ice {
            columns.push(Planets::Ice);
            values.push(ice.into());
            conflict.update_column(Planets::Ice);
        }

        if let Some(note) = note {
            columns.push(Planets::Note);
            values.push(note.into());
            conflict.update_column(Planets::Note);
        }

        columns.push(Planets::IsMoon);
        values.push(is_moon.into());
        conflict.update_column(Planets::IsMoon);

        sea_query::Query::insert()
            .into_table(Planets::Table)
            .columns(columns)
            .values(values)?
            .on_conflict(conflict)
            .build(sea_query::SqliteQueryBuilder)
    };

    let turso_params: Vec<turso::Value> = query_values
        .into_iter()
        .map(|v| match v {
            sea_query::Value::Int(Some(i)) => turso::Value::Integer(i as i64),
            sea_query::Value::BigInt(Some(i)) => turso::Value::Integer(i),
            sea_query::Value::Double(Some(f)) => turso::Value::Real(f),
            sea_query::Value::String(Some(s)) => turso::Value::Text(s),
            sea_query::Value::Bool(Some(b)) => turso::Value::Integer(if b { 1 } else { 0 }),
            _ => turso::Value::Null,
        })
        .collect();

    conn.execute(&sql, turso_params).await?;

    Ok(())
}

fn construct_planet(row: &Row) -> anyhow::Result<Planet> {
    let mut idx = 0;

    Ok(Planet {
        id: row.get(idx)?,
        star_id: row.get({
            idx += 1;
            idx
        })?,
        name: row.get({
            idx += 1;
            idx
        })?,
        radius: row.get({
            idx += 1;
            idx
        })?,
        gravity: row.get({
            idx += 1;
            idx
        })?,
        conditions: row.get({
            idx += 1;
            idx
        })?,
        temperature: row.get({
            idx += 1;
            idx
        })?,
        sector: row.get({
            idx += 1;
            idx
        })?,
        tectonics: row.get({
            idx += 1;
            idx
        })?,
        atmosphere: row.get({
            idx += 1;
            idx
        })?,
        oceans: row.get({
            idx += 1;
            idx
        })?,
        rings: row.get({
            idx += 1;
            idx
        })?,
        trees: row.get({
            idx += 1;
            idx
        })?,
        sub_trees: row.get({
            idx += 1;
            idx
        })?,
        life: row.get::<i64>({
            idx += 1;
            idx
        })? != 0,
        life_type: row.get({
            idx += 1;
            idx
        })?,
        is_moon: row.get::<i64>({
            idx += 1;
            idx
        })? != 0,
        moons: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        malachite: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        hematite: row.get({
            idx += 1;
            idx
        })?,
        petroleum: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        coal: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        gummite: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        tektite: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        bauxite: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        gold: row.get({
            idx += 1;
            idx
        })?,
        cerussite: row
            .get::<Option<i32>>({
                idx += 1;
                idx
            })?
            .map(|v| v),
        lime: row
            .get::<Option<i64>>({
                idx += 1;
                idx
            })?
            .map(|v| v != 0),
        saltpeter: row
            .get::<Option<i64>>({
                idx += 1;
                idx
            })?
            .map(|v| v != 0),
        quartz: row
            .get::<Option<i64>>({
                idx += 1;
                idx
            })?
            .map(|v| v != 0),
        ice: row
            .get::<Option<i64>>({
                idx += 1;
                idx
            })?
            .map(|v| v != 0),
        note: row.get({
            idx += 1;
            idx
        })?,
    })
}

fn check_sql(input: &str, state: AppState) -> bool {
    let upper = input.to_uppercase();
    state
        .configs
        .sql_blacklist
        .iter()
        .any(|sql| upper.contains(sql))
}

fn check_index(index: &str) -> anyhow::Result<()> {
    match index.split_once("-") {
        Some((num1, num2)) => {
            num1.parse::<i64>()
                .map_err(|_| anyhow::anyhow!("No star id"))?;
            if let Err(_) = num2.parse::<i64>() {
                let mut split = num2.split("-");
                split
                    .by_ref()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No planet id"))?
                    .parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Blacklisted sql"))?;
                split
                    .by_ref()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No moon id"))?
                    .parse::<i64>()
                    .map_err(|_| anyhow::anyhow!("Blacklisted sql"))?;

                if split.next().is_some() {
                    anyhow::bail!("Blacklisted sql")
                }
            }
            Ok(())
        }
        None => anyhow::bail!("Blacklisted sql"),
    }
}

fn validate(key: &str, value: &str) -> anyhow::Result<()> {
    match key {
        "malachite" | "hematite" | "petroleum" | "coal" | "gummite" | "tektite" | "bauxite"
        | "gold" | "cerussite" => {
            let concentration = value.parse::<f64>()?;
            if concentration < 0.0 || concentration > 3.0 {
                anyhow::bail!("Wrong concentration information in {}", key)
            }
        }
        "life" | "lime" | "saltpeter" | "quartz" | "ice" => {
            if value != "true" && value != "false" {
                anyhow::bail!("{} is supposed to have true/false value", key)
            }
        }
        // will be improved later
        "sector" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .sectors
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Sector type '{}' isn't in the game", value))
            }
        }
        "tectonics" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .tectonics
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tectonic type '{}' isn't in the game", value))
            }
        }
        "atmosphere" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .atmospheres
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Atmospheric type '{}' isn't in the game", value))
            }
        }
        /*"oceans" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .oceans
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Oceans type '{}' isn't in the game", value))
            }
        }*/
        /*"trees" | "sub trees" => {
            if !CHECKS
                .get()
                .ok_or(anyhow::anyhow!("CHECKS isn't initialized"))?
                .trees
                .contains(&value.to_string())
            {
                anyhow::bail!(format!("Tree type '{}' isn't in the game", value))
            }
        }*/
        _ => {}
    }

    Ok(())
}

fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains("&&") || input.contains("||") {
        Cow::Owned(input.replace("&&", "and").replace("||", "or"))
    } else {
        Cow::Borrowed(input)
    }
}