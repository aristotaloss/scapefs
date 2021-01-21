use std::{collections::HashMap, convert::TryInto};
use std::io::{Read, Seek};
use byteorder::{ReadBytesExt, BigEndian};

#[derive(Clone, Debug, Default)]
pub struct ReferenceTable {
	version: u8,
	revision: u32,
    flags: ReferenceTableFlags,
    
    entries: HashMap<i32, ReferenceTableFolder>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct ReferenceTableFlags {
	has_names: bool,
	has_whirlpool: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ReferenceTableFolder {
	id: i32,
	name_hash: i32,
	crc32: i32,
	whirlpool: Vec<u8>,
	version: u32,
	files: HashMap<i32, ReferenceTableFile>,

	// Internal only, used when decoding
	file_count: i32,
}
impl ReferenceTableFolder {
    pub fn new(id: i32) -> ReferenceTableFolder {
        ReferenceTableFolder {
            id: id,
            name_hash: 0,
            crc32: 0,
            whirlpool: Vec::new(),
            version: 0,
            files: HashMap::new(),
            file_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceTableFile {
	id: i32,
	name_hash: i32,
}

trait VarIntRead {
    fn read_vari32(&mut self) -> Result<i32, std::io::Error>;
}

impl<R: Read + Seek> VarIntRead for R {
    fn read_vari32(&mut self) -> Result<i32, std::io::Error> {
        let first = self.read_i8()?;

        // Unseek back to where we were
        self.seek(std::io::SeekFrom::Current(-1))?;

        // If bit 8 is set (sign bit), the low 31 bits
        // of the current 4 bytes represent an int32, otherwise
        // they represent an int16.
        if first < 0 {
            Ok(self.read_i32::<BigEndian>()? & 0x7FFFFFFF)
        } else {
            Ok(self.read_i16::<BigEndian>()?.into())
        }
    }
}

impl ReferenceTable {

    pub fn decode<R: Read + Seek>(r: &mut R) -> Result<ReferenceTable, std::io::Error> {
        let mut table = ReferenceTable::default();

        table.version = r.read_u8()?;

        if table.version >= 5 && table.version <= 7 {
            if table.version >= 6 {
                table.revision = r.read_u32::<BigEndian>()?;
            }

            let flags = r.read_u8()?;
            table.flags.has_names = (flags & 0x1) != 0;
            table.flags.has_whirlpool = (flags & 0x2) != 0;

            // These are not yet identified (present in higher revisions)
            let _unknown1 = (flags & 0x4) != 0;
            let _unknown2 = (flags & 0x8) != 0;

            let entry_count: u32;
            if table.version >= 7 {
                entry_count = r.read_vari32()?.try_into().unwrap();
            } else {
                entry_count = r.read_u16::<BigEndian>()?.into();
            }

            // Translation table maps array indices to actual IDs
            let mut entries = Vec::<ReferenceTableFolder>::with_capacity(entry_count.try_into().unwrap());

            let mut id = 0;
            for _ in 0..(entry_count as usize) {
                // Type of data depends on the table version - only 7+ supports >65535
                if table.version >= 7 {
                    id += r.read_vari32()?;
                } else {
                    id += r.read_u16::<BigEndian>()? as i32;
                }

                entries.push(ReferenceTableFolder::new(id));
            }

            // Load all names, if present
            if table.flags.has_names {
                for i in 0..entry_count {
                    entries[i as usize].name_hash = r.read_i32::<BigEndian>()?;
                }
            }

            // Load CRC values
            for i in 0..entry_count {
                entries[i as usize].crc32 = r.read_i32::<BigEndian>()?;
            }

            // Unidentified
            if _unknown2 {
                for _i in 0..entry_count {
                    r.read_i32::<BigEndian>()?;
                }
            }

            // Read whirlpool values
            if table.flags.has_whirlpool {
                for i in 0..entry_count {
                    r.read_exact(entries[i as usize].whirlpool.as_mut_slice())?;
                }
            }

            // Unidentified
            if _unknown1 {
                for _i in 0..entry_count {
                    r.read_i32::<BigEndian>()?;
                    r.read_i32::<BigEndian>()?;
                }
            }

            // Load folder versions
            for i in 0..entry_count {
                entries[i as usize].version = r.read_u32::<BigEndian>()?;
            }

            let mut files = Vec::<Vec<ReferenceTableFile>>::with_capacity(entry_count.try_into().unwrap());

            // Load file counts
            for _ in 0..entry_count {
                let file_count;

                if table.version >= 7 {
                    file_count = r.read_vari32()?;
                } else {
                    file_count = r.read_u16::<BigEndian>()? as i32;
                }

                files.push(Vec::<ReferenceTableFile>::with_capacity(file_count as usize));
            }

            // Load file IDs
            for i in 0..entry_count {
                let mut file_id = 0;

                for _ in 0..files[i as usize].len() {
                    if table.version >= 7 {
                        file_id += r.read_vari32()?;
                    } else {
                        file_id += r.read_u16::<BigEndian>()? as i32;
                    }

                    let mut file = ReferenceTableFile::default();
                    file.id = file_id;
                    files[i as usize].push(file);
                }
            }

            // Load file names
            if table.flags.has_names {
                for i in 0..entry_count as usize {
                    for file in 0..files[i as usize].len() {
                        files[i][file].name_hash = r.read_i32::<BigEndian>()?;
                    }
                }
            }

            // Turn the entry array into a lookup map
            table.entries = HashMap::with_capacity(entry_count as usize);
            for (i, v) in entries.iter_mut().enumerate() {
                // Turn the children into lookup maps too
                v.files = HashMap::with_capacity(files[i].len() as usize);

                for file in &files[i] {
                    v.files.insert(file.id, *file);
                }

                table.entries.insert(v.id, v.clone());
            }

            Ok(table)
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid reference table version"))
        }
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn lookup(&self, id: i32) -> Option<&ReferenceTableFolder> {
        self.entries.get(&id)
    }

    pub fn lookup_mut(&mut self, id: i32) -> Option<&mut ReferenceTableFolder> {
        self.entries.get_mut(&id)
    }
    
    pub fn last_id(&self) -> i32 {
        let mut last_id = 0;
    
        for (_, v) in &self.entries {
            if v.id > last_id {
                last_id = v.id
            }
        }
    
        return last_id
    }

}
