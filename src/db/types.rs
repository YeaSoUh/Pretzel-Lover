use std::{fmt::Display, sync::Arc, time::Duration};

use serde::Deserialize;
use turso::{Connection, Database, Row};

#[derive(Debug)]
pub struct EditRequest {
    pub input: String,
    pub index: String,
}

#[derive(Debug)]
pub struct PlanetResult {
    pub planet: Planet,
    pub result: Row,
    pub(in crate::db) prettier: bool,
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
pub(in crate::db) struct DatabaseStruct {
    pub(crate) database: Arc<Database>,
}

impl DatabaseStruct {
    pub async fn get_conn(&self) -> anyhow::Result<Connection> {
        let db_ref = Arc::clone(&self.database);

        let conn = db_ref.as_ref().connect()?;
        conn.busy_timeout(Duration::from_millis(500))?; // 0.5 seconds
        conn.pragma_update("journal_mode", "'mvcc'").await?; // enables concurrency writes which is good!

        Ok(conn)
    }
}

#[derive(Deserialize)]
pub(in crate::db) struct Checks {
    #[serde(rename = "allowed_sectors")]
    pub(in crate::db) sectors: Vec<String>,
    #[serde(rename = "allowed_tectonics")]
    pub(in crate::db) tectonics: Vec<String>,
    #[serde(rename = "allowed_atmospheres")]
    pub(in crate::db) atmospheres: Vec<String>,
    #[serde(skip)]
    #[serde(rename = "allowed_oceans")]
    #[allow(dead_code)]
    pub(in crate::db) oceans: Vec<String>,
    #[serde(skip)]
    #[serde(rename = "allowed_trees")]
    #[allow(dead_code)]
    pub(in crate::db) trees: Vec<String>, // won't be used
}

#[derive(sea_query::Iden)]
pub(in crate::db) enum Planets {
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
#[derive(Deserialize, Debug)]
pub struct Planet {
    pub id: String,
    pub star_id: i64,
    // default: "Unknown"
    pub name: String,
    // default: 0 for radius, gravity and temp
    pub radius: f64,
    pub gravity: f64,
    // default: "Unknown"
    pub conditions: String,
    pub temperature: i64,
    pub sector: Option<String>,
    // Default: "Unknown" for tectonics
    pub tectonics: String,
    pub atmosphere: Option<String>,
    pub oceans: Option<String>,
    pub rings: Option<String>,
    pub trees: Option<String>,
    pub sub_trees: Option<String>,
    // default: false (only for life)
    pub life: bool,
    pub life_type: Option<String>,
    pub is_moon: bool,
    pub moons: Option<i32>,

    pub malachite: Option<i32>,
    pub hematite: Option<f64>,
    pub petroleum: Option<i32>,
    pub coal: Option<i32>,
    pub gummite: Option<i32>,
    pub tektite: Option<i32>,
    pub bauxite: Option<i32>,
    pub gold: Option<f64>,
    pub cerussite: Option<i32>,

    pub lime: Option<bool>,
    pub saltpeter: Option<bool>,
    pub quartz: Option<bool>,
    pub ice: Option<bool>,

    pub note: Option<String>,
}

impl TryFrom<&Row> for Planet {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
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
}

impl TryFrom<Row> for Planet {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Planet::try_from(&row)
    }
}