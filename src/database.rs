use anyhow::Result;
use duckdb::{params, Connection};
use std::path::Path;

/// Satellite details stored in the database
#[derive(Debug, Clone, Default)]
pub struct SatelliteDetails {
    pub id: Option<i64>,
    pub name: String,
    pub tle_line1: String,
    pub tle_line2: String,
    pub launch_date: Option<String>,
    pub launch_site: Option<String>,
    pub country_of_origin: Option<String>,
    pub operator: Option<String>,
    pub satellite_type: Option<String>,
    pub downlink_frequency_mhz: Option<f64>,
    pub uplink_frequency_mhz: Option<f64>,
    pub notes: Option<String>,
}

impl SatelliteDetails {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            tle_line1: String::new(),
            tle_line2: String::new(),
            launch_date: None,
            launch_site: None,
            country_of_origin: None,
            operator: None,
            satellite_type: None,
            downlink_frequency_mhz: None,
            uplink_frequency_mhz: None,
            notes: None,
        }
    }
}

/// Database manager for satellite details
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (useful for testing)
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS satellite_details (
                id INTEGER PRIMARY KEY,
                name VARCHAR NOT NULL UNIQUE,
                tle_line1 VARCHAR DEFAULT '',
                tle_line2 VARCHAR DEFAULT '',
                launch_date VARCHAR,
                launch_site VARCHAR,
                country_of_origin VARCHAR,
                operator VARCHAR,
                satellite_type VARCHAR,
                downlink_frequency_mhz DOUBLE,
                uplink_frequency_mhz DOUBLE,
                notes VARCHAR
            );
            "#,
        )?;
        Ok(())
    }

    /// Create a new satellite details entry
    pub fn create(&self, details: &SatelliteDetails) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO satellite_details (
                name, tle_line1, tle_line2, launch_date, launch_site,
                country_of_origin, operator, satellite_type,
                downlink_frequency_mhz, uplink_frequency_mhz, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                details.name,
                details.tle_line1,
                details.tle_line2,
                details.launch_date,
                details.launch_site,
                details.country_of_origin,
                details.operator,
                details.satellite_type,
                details.downlink_frequency_mhz,
                details.uplink_frequency_mhz,
                details.notes,
            ],
        )?;

        // Get the last inserted row id
        let id: i64 = self.conn.query_row(
            "SELECT last_insert_rowid()",
            [],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    /// Read satellite details by name
    pub fn read_by_name(&self, name: &str) -> Result<Option<SatelliteDetails>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, tle_line1, tle_line2, launch_date, launch_site,
                   country_of_origin, operator, satellite_type,
                   downlink_frequency_mhz, uplink_frequency_mhz, notes
            FROM satellite_details
            WHERE name = ?
            "#,
        )?;

        let result = stmt.query_row(params![name], |row| {
            Ok(SatelliteDetails {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                tle_line1: row.get(2)?,
                tle_line2: row.get(3)?,
                launch_date: row.get(4)?,
                launch_site: row.get(5)?,
                country_of_origin: row.get(6)?,
                operator: row.get(7)?,
                satellite_type: row.get(8)?,
                downlink_frequency_mhz: row.get(9)?,
                uplink_frequency_mhz: row.get(10)?,
                notes: row.get(11)?,
            })
        });

        match result {
            Ok(details) => Ok(Some(details)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Read satellite details by ID
    pub fn read_by_id(&self, id: i64) -> Result<Option<SatelliteDetails>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, tle_line1, tle_line2, launch_date, launch_site,
                   country_of_origin, operator, satellite_type,
                   downlink_frequency_mhz, uplink_frequency_mhz, notes
            FROM satellite_details
            WHERE id = ?
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(SatelliteDetails {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                tle_line1: row.get(2)?,
                tle_line2: row.get(3)?,
                launch_date: row.get(4)?,
                launch_site: row.get(5)?,
                country_of_origin: row.get(6)?,
                operator: row.get(7)?,
                satellite_type: row.get(8)?,
                downlink_frequency_mhz: row.get(9)?,
                uplink_frequency_mhz: row.get(10)?,
                notes: row.get(11)?,
            })
        });

        match result {
            Ok(details) => Ok(Some(details)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Read all satellite details
    pub fn read_all(&self) -> Result<Vec<SatelliteDetails>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, tle_line1, tle_line2, launch_date, launch_site,
                   country_of_origin, operator, satellite_type,
                   downlink_frequency_mhz, uplink_frequency_mhz, notes
            FROM satellite_details
            ORDER BY name
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SatelliteDetails {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                tle_line1: row.get(2)?,
                tle_line2: row.get(3)?,
                launch_date: row.get(4)?,
                launch_site: row.get(5)?,
                country_of_origin: row.get(6)?,
                operator: row.get(7)?,
                satellite_type: row.get(8)?,
                downlink_frequency_mhz: row.get(9)?,
                uplink_frequency_mhz: row.get(10)?,
                notes: row.get(11)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Update satellite details
    pub fn update(&self, details: &SatelliteDetails) -> Result<bool> {
        let id = match details.id {
            Some(id) => id,
            None => return Ok(false),
        };

        let affected = self.conn.execute(
            r#"
            UPDATE satellite_details SET
                name = ?,
                tle_line1 = ?,
                tle_line2 = ?,
                launch_date = ?,
                launch_site = ?,
                country_of_origin = ?,
                operator = ?,
                satellite_type = ?,
                downlink_frequency_mhz = ?,
                uplink_frequency_mhz = ?,
                notes = ?
            WHERE id = ?
            "#,
            params![
                details.name,
                details.tle_line1,
                details.tle_line2,
                details.launch_date,
                details.launch_site,
                details.country_of_origin,
                details.operator,
                details.satellite_type,
                details.downlink_frequency_mhz,
                details.uplink_frequency_mhz,
                details.notes,
                id,
            ],
        )?;

        Ok(affected > 0)
    }

    /// Delete satellite details by ID
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM satellite_details WHERE id = ?",
            params![id],
        )?;
        Ok(affected > 0)
    }

    /// Delete satellite details by name
    pub fn delete_by_name(&self, name: &str) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM satellite_details WHERE name = ?",
            params![name],
        )?;
        Ok(affected > 0)
    }

    /// Insert or update (upsert) satellite details by name
    pub fn upsert(&self, details: &SatelliteDetails) -> Result<i64> {
        // Check if satellite exists
        if let Some(existing) = self.read_by_name(&details.name)? {
            let mut updated = details.clone();
            updated.id = existing.id;
            self.update(&updated)?;
            Ok(existing.id.unwrap())
        } else {
            self.create(details)
        }
    }

    /// Get count of satellites in database
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM satellite_details",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_read() {
        let db = Database::open_in_memory().unwrap();

        let details = SatelliteDetails {
            id: None,
            name: "ISS (ZARYA)".to_string(),
            tle_line1: "1 25544U 98067A   24001.50000000  .00000000  00000-0  00000-0 0    09".to_string(),
            tle_line2: "2 25544  51.6400 297.8590 0001218  88.9267 338.1310 15.49000000    04".to_string(),
            launch_date: Some("1998-11-20".to_string()),
            launch_site: Some("Baikonur Cosmodrome".to_string()),
            country_of_origin: Some("Russia/USA".to_string()),
            operator: Some("NASA/Roscosmos".to_string()),
            satellite_type: Some("Space Station".to_string()),
            downlink_frequency_mhz: Some(145.800),
            uplink_frequency_mhz: Some(145.990),
            notes: Some("Test notes".to_string()),
        };

        let id = db.create(&details).unwrap();
        assert!(id > 0);

        let read = db.read_by_name("ISS (ZARYA)").unwrap().unwrap();
        assert_eq!(read.name, "ISS (ZARYA)");
        assert_eq!(read.launch_date, Some("1998-11-20".to_string()));
        assert_eq!(read.downlink_frequency_mhz, Some(145.800));
    }

    #[test]
    fn test_update() {
        let db = Database::open_in_memory().unwrap();

        let mut details = SatelliteDetails::new("TEST SAT".to_string());
        details.country_of_origin = Some("USA".to_string());

        let id = db.create(&details).unwrap();
        details.id = Some(id);
        details.country_of_origin = Some("Germany".to_string());

        assert!(db.update(&details).unwrap());

        let read = db.read_by_id(id).unwrap().unwrap();
        assert_eq!(read.country_of_origin, Some("Germany".to_string()));
    }

    #[test]
    fn test_delete() {
        let db = Database::open_in_memory().unwrap();

        let details = SatelliteDetails::new("TO DELETE".to_string());
        let id = db.create(&details).unwrap();

        assert!(db.delete(id).unwrap());
        assert!(db.read_by_id(id).unwrap().is_none());
    }
}
