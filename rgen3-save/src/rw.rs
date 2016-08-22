use std::io::prelude::*;
use std::io::{self, SeekFrom};
use byteorder::{LittleEndian as LE, ReadBytesExt, WriteBytesExt};
use std::error::Error;
use util::LowerUpper;
use {TrainerInfo, DATA_SIZE, rgen3_string, Time, Section, SectionData, SaveBlock, Save, Gender,
     UNKNOWN_SAVE_FOOTER_SIZE, TRAINER_INFO_UNKNOWN_3_SIZE, Game,
     RS_EM_PLAYERINFO_TRAILING_DATA_SIZE, FRLG_PLAYERINFO_TRAILING_DATA_SIZE,
     FRLG_PLAYERINFO_UNKNOWN_CHUNK_SIZE, TeamAndItems, GameType, TeamAndItemsUnknown,
     EM_RU_SA_TEAMANDITEMS_UNK_LEN, FR_LG_TEAMANDITEMS_UNK_LEN, Pokemon, POKEMON_NICK_LEN,
     TRAINER_NAME_LEN, PokemonData, PokemonGrowth, PokemonAttacks, PokemonEvsAndCondition,
     PokemonMisc, TrainerName, PokemonNick, TeamAndItemsRemaining, EM_RU_SA_TEAMANDITEMS_REM_LEN,
     FR_LG_TEAMANDITEMS_REM_LEN, TEAMANDITEMS_POKE_LEN, PcBuffer, PokemonStorage, N_BOXES,
     PokemonActiveData, PokeBox};

trait SectionWrite {
    fn id(&self) -> u16;
    fn cksum_area_len(&self) -> u64;
    fn write_data<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut buf = Vec::new();
        self.write_data(&mut buf)?;
        // Write section id
        buf.write_u16::<LE>(self.id())?;
        // Calculate and write checksum
        let mut cksum: u32 = 0;
        let mut reader = &buf[..];
        let mut amount_read = 0;

        while amount_read < self.cksum_area_len() {
            cksum = cksum.wrapping_add(reader.read_u32::<LE>()?);
            amount_read += 4;
        }
        writer.write_all(&buf)?;
        let (lower, upper) = cksum.split();
        let cksum = (upper as u16).wrapping_add(lower as u16);
        debug!("Calculated checksum is {}", cksum);
        writer.write_u16::<LE>(cksum)
    }
}

impl TrainerInfo {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        let mut name_buffer = [0u8; 7];
        reader.read_exact(&mut name_buffer)?;
        debug!("Trainer name is {}",
               rgen3_string::decode_string(&name_buffer));
        let unknown_1 = reader.read_u8()?;
        let gender = match reader.read_u8()? {
            0 => Gender::Male,
            1 => Gender::Female,
            etc => return Err(format!("Invalid gender value: {}", etc).into()),
        };
        debug!("Trainer gender is {}", gender);
        let unknown_2 = reader.read_u8()?;
        let id = reader.read_u32::<LE>()?;
        let (public_id, secret_id) = id.split();
        debug!("({}) Public id: {}, secret id: {}",
               id,
               public_id,
               secret_id);
        let time_played = Time::read(reader)?;
        debug!("Time played: {:05}:{:02}:{:02}:{:02}",
               time_played.hours,
               time_played.minutes,
               time_played.seconds,
               time_played.frames);
        let mut options_data = [0u8; 3];
        reader.read_exact(&mut options_data)?;
        let mut unknown_3 = [0u8; TRAINER_INFO_UNKNOWN_3_SIZE];
        reader.read_exact(&mut unknown_3)?;
        let game = Game::read(reader)?;
        debug!("Game info: {:?}", game);
        Ok(TrainerInfo {
            name: TrainerName(name_buffer),
            unknown_1: unknown_1,
            gender: gender,
            unknown_2: unknown_2,
            public_id: public_id as u16,
            secret_id: secret_id as u16,
            time_played: time_played,
            options_data: options_data,
            unknown_3: unknown_3,
            game: game,
        })
    }
}

impl SectionWrite for TrainerInfo {
    fn id(&self) -> u16 {
        0
    }
    fn cksum_area_len(&self) -> u64 {
        3884
    }
    fn write_data<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.name.0)?;
        writer.write_u8(self.unknown_1)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u8(self.unknown_2)?;
        writer.write_u32::<LE>(LowerUpper::merge(self.public_id, self.secret_id))?;
        self.time_played.write(writer)?;
        writer.write_all(&self.options_data)?;
        writer.write_all(&self.unknown_3)?;
        self.game.write(writer)
    }
}

impl Time {
    fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        Ok(Time {
            hours: reader.read_u16::<LE>()?,
            minutes: reader.read_u8()?,
            seconds: reader.read_u8()?,
            frames: reader.read_u8()?,
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LE>(self.hours)?;
        writer.write_u8(self.minutes)?;
        writer.write_u8(self.seconds)?;
        writer.write_u8(self.frames)
    }
}

#[derive(Default)]
struct ReadSession {
    game_type: Option<GameType>,
    save_index: Option<u32>,
    section_index: usize,
    trainer_info_index: Option<usize>,
    team_and_items_index: Option<usize>,
    nonexistent: bool,
    box_indexes: [usize; N_BOXES],
}

impl Section {
    fn read<R: Read + Seek>(reader: &mut R, session: &mut ReadSession) -> Result<Self, Box<Error>> {
        debug!("== Reading section at offset {} ==",
               reader.seek(SeekFrom::Current(0))?);
        // Skip data, so we can read section info first
        let data_pos = reader.seek(SeekFrom::Current(0))?;
        reader.seek(SeekFrom::Current(DATA_SIZE))?;
        let id = reader.read_u16::<LE>()?;
        let cksum = reader.read_u16::<LE>()?;
        let unknown_1 = reader.read_u32::<LE>()?;
        let save_idx = reader.read_u32::<LE>()?;
        match session.save_index {
            ref mut opt @ None => *opt = Some(save_idx),
            Some(index) => {
                if save_idx != index {
                    return Err(format!("Not all save indexes in a block are the same. prev: {}, \
                                        now: {}",
                                       index,
                                       save_idx)
                        .into());
                }
            }
        }
        debug!("Section id: {}, cksum: {}, save idx: {}",
               id,
               cksum,
               save_idx);
        // Go ahead and read the data now
        let section_end_pos = reader.seek(SeekFrom::Current(0))?;
        reader.seek(SeekFrom::Start(data_pos))?;
        let data = match id {
            0 => {
                let info = TrainerInfo::read(reader)?;
                session.game_type = Some(GameType::from(&info.game));
                match session.trainer_info_index {
                    ref mut opt @ None => *opt = Some(session.section_index),
                    Some(idx) => {
                        return Err(format!("Duplicate TrainerInfo section at index {}. Previous \
                                            was at index {}.",
                                           session.section_index,
                                           idx)
                            .into())
                    }
                }
                SectionData::TrainerInfo(info)
            }
            1 => {
                match session.team_and_items_index {
                    ref mut opt @ None => *opt = Some(session.section_index),
                    Some(idx) => {
                        return Err(format!("Duplicate TeamAndItems section at index {}. Previous \
                                            was at index {}.",
                                           session.section_index,
                                           idx)
                            .into())
                    }
                }
                SectionData::TeamAndItems(TeamAndItems::read(reader, session)?)
            }
            5...13 => {
                let index = id as usize - 5;
                session.box_indexes[index] = session.section_index;
                SectionData::PcBuffer(PcBuffer::read(reader, index)?)
            }
            0xFFFF => {
                session.nonexistent = true;
                SectionData::Unimplemented {
                    raw: [0xFF; DATA_SIZE as usize],
                    id: id,
                    cksum: 0xFFFF,
                }
            }
            _ => {
                // unimplemented section, just save the raw data
                let mut data = [0u8; DATA_SIZE as usize];
                reader.read_exact(&mut data)?;
                SectionData::Unimplemented {
                    raw: data,
                    id: id,
                    cksum: cksum,
                }
            }
        };
        // Return to end of section
        reader.seek(SeekFrom::Start(section_end_pos))?;
        // Increment section index counter
        session.section_index += 1;
        Ok(Section {
            data: data,
            unknown_1: unknown_1,
            save_idx: save_idx,
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.data.write(writer)?;
        writer.write_u32::<LE>(self.unknown_1)?;
        writer.write_u32::<LE>(self.save_idx)
    }
}

impl SectionData {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match *self {
            SectionData::Unimplemented { ref raw, id, cksum } => {
                writer.write_all(raw)?;
                assert_eq!(raw.len(), DATA_SIZE as usize);
                writer.write_u16::<LE>(id)?;
                writer.write_u16::<LE>(cksum)
            }
            SectionData::TrainerInfo(ref info) => info.write(writer),
            SectionData::TeamAndItems(ref data) => data.write(writer),
            SectionData::PcBuffer(ref data) => data.write(writer),
        }
    }
}

struct PokemonStorageReader<'a> {
    sections: &'a [Section],
    session: &'a ReadSession,
    read_so_far: u64,
}

impl<'a> PokemonStorageReader<'a> {
    fn new(sections: &'a [Section], session: &'a ReadSession) -> Self {
        PokemonStorageReader {
            sections: sections,
            session: session,
            read_so_far: 0,
        }
    }
}

const PC_BUFFER_DATA_LEN: usize = 3968;

impl<'a> Read for PokemonStorageReader<'a> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        debug!("PokemonStorageReader::read called on {:?}", buf);
        debug!("Read so far: {}", self.read_so_far);
        let index = self.read_so_far / PC_BUFFER_DATA_LEN as u64;
        debug!("Box section to read from is {}", index);
        let cursor = (self.read_so_far % PC_BUFFER_DATA_LEN as u64) as usize;
        debug!("Offset is {}", index);
        let box_index = self.session.box_indexes[index as usize];
        let pc_buf = if let SectionData::PcBuffer(ref buf) = self.sections[box_index].data {
            buf
        } else {
            panic!("Pc Buffer session expected, got some other section.");
        };
        let pc_buf_data = &pc_buf.data[cursor..buf.len() + cursor];
        debug!("PC buf data: {:?}", pc_buf_data);
        debug!("Witebuf len is {}, src len is {}",
               buf.len(),
               pc_buf_data.len());
        buf.copy_from_slice(pc_buf_data);
        self.read_so_far += buf.len() as u64;
        Ok(buf.len())
    }
}

struct PokemonStorageWriter<'a> {
    sections: &'a mut [Section],
    written_so_far: u64,
    box_indexes: [usize; N_BOXES],
}

impl<'a> Write for PokemonStorageWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        debug!("PokemonStorageWriter::write called on {:?}", buf);
        debug!("Written so far: {}", self.written_so_far);
        let index = self.written_so_far / PC_BUFFER_DATA_LEN as u64;
        debug!("Box section to read from is {}", index);
        let cursor = (self.written_so_far % PC_BUFFER_DATA_LEN as u64) as usize;
        debug!("Offset is {}", index);
        let box_index = self.box_indexes[index as usize];
        let pc_buf = if let SectionData::PcBuffer(ref mut buf) = self.sections[box_index].data {
            buf
        } else {
            panic!("Pc Buffer section expected, got {:?}",
                   self.sections[box_index]);
        };
        let pc_buf_data = &mut pc_buf.data[cursor..buf.len() + cursor];
        debug!("PC buf data: {:?}", pc_buf_data);
        debug!("Witebuf len is {}, src len is {}",
               buf.len(),
               pc_buf_data.len());
        pc_buf_data.copy_from_slice(buf);
        self.written_so_far += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SaveBlock {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<(Self, u32), Box<Error>> {
        debug!("== Reading save block ==");
        let mut session = ReadSession::default();
        let sections = [Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?,
                        Section::read(reader, &mut session)?];
        let (trainer_info_index, team_and_items_index, storage);
        if session.nonexistent {
            trainer_info_index = 0;
            team_and_items_index = 0;
            storage = Default::default();
        } else {
            trainer_info_index = session.trainer_info_index.ok_or("Missing TrainerInfo section")?;
            team_and_items_index = session.team_and_items_index
                .ok_or("Missing TeamAndItems section")?;
            let mut reader = PokemonStorageReader::new(&sections, &session);
            storage = PokemonStorage::read(&mut reader)?;
        }
        Ok((SaveBlock {
            sections: sections,
            trainer_info_index: trainer_info_index,
            team_and_items_index: team_and_items_index,
            nonexistent: session.nonexistent,
            pokemon_storage: storage,
            box_indexes: session.box_indexes,
        },
            session.save_index.unwrap()))
    }
    fn write<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        {
            let mut storage_writer = PokemonStorageWriter {
                sections: &mut self.sections,
                written_so_far: 0,
                box_indexes: self.box_indexes,
            };
            if !self.nonexistent {
                self.pokemon_storage.write(&mut storage_writer)?;
            }
        }
        for sec in &self.sections {
            sec.write(writer)?;
        }
        Ok(())
    }
}

impl Save {
    /// Read the save data from a `Read` implementer.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Box<Error>> {
        debug!("== Reading save ==");
        let (block1, block1_idx) = SaveBlock::read(reader)?;
        let (block2, block2_idx) = SaveBlock::read(reader)?;
        let mut unknown = [0; UNKNOWN_SAVE_FOOTER_SIZE];
        reader.read_exact(&mut unknown)?;
        let most_recent_index = if !block1.nonexistent && !block2.nonexistent {
            if block1_idx > block2_idx { 0 } else { 1 }
        } else if !block1.nonexistent && block2.nonexistent {
            0
        } else if !block2.nonexistent && block1.nonexistent {
            1
        } else {
            panic!("Both block 1 and block 2 do not exist.")
        };
        Ok(Save {
            blocks: [block1, block2],
            unknown: unknown,
            most_recent_index: most_recent_index,
        })
    }
    /// Write the save data to a `Write` implementer.
    pub fn write<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        for block in &mut self.blocks {
            block.write(writer)?
        }
        writer.write_all(&self.unknown)
    }
}

impl Game {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        Ok(match reader.read_u32::<LE>()? {
            0 => {
                let mut trailing_data = [0; RS_EM_PLAYERINFO_TRAILING_DATA_SIZE];
                reader.read_exact(&mut trailing_data)?;
                Game::RubyOrSapphire { trailing_data: trailing_data }
            }
            1 => {
                let mut unknown = [0; FRLG_PLAYERINFO_UNKNOWN_CHUNK_SIZE];
                reader.read_exact(&mut unknown)?;
                let security_key = reader.read_u32::<LE>()?;
                let mut trailing_data = [0; FRLG_PLAYERINFO_TRAILING_DATA_SIZE];
                reader.read_exact(&mut trailing_data)?;
                Game::FireredOrLeafgreen {
                    unknown: unknown,
                    security_key: security_key,
                    trailing_data: trailing_data,
                }
            }
            etc => {
                let mut trailing_data = [0; RS_EM_PLAYERINFO_TRAILING_DATA_SIZE];
                reader.read_exact(&mut trailing_data)?;
                Game::Emerald {
                    security_key: etc,
                    trailing_data: trailing_data,
                }
            }
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match *self {
            Game::RubyOrSapphire { ref trailing_data } => {
                writer.write_u32::<LE>(0)?;
                writer.write_all(trailing_data)
            }
            Game::FireredOrLeafgreen { ref unknown, security_key, ref trailing_data } => {
                writer.write_u32::<LE>(1)?;
                writer.write_all(unknown)?;
                writer.write_u32::<LE>(security_key)?;
                writer.write_all(trailing_data)
            }
            Game::Emerald { security_key, ref trailing_data } => {
                writer.write_u32::<LE>(security_key)?;
                writer.write_all(trailing_data)
            }
        }
    }
}

impl TeamAndItems {
    fn read<R: Read>(reader: &mut R, session: &ReadSession) -> Result<Self, Box<Error>> {
        let game_type = session.game_type.expect("Game type not yet available");
        let unknown = TeamAndItemsUnknown::read(reader, game_type)?;
        let team_size = reader.read_u32::<LE>()?;
        debug!("Team size is {}", team_size);
        let mut poke_data = [0; TEAMANDITEMS_POKE_LEN];
        reader.read_exact(&mut poke_data)?;
        let mut poke_reader = &poke_data[..];
        let mut team = Vec::new();
        for _ in 0..team_size {
            team.push(Pokemon::read(&mut poke_reader)?);
        }
        let remaining = TeamAndItemsRemaining::read(reader, game_type)?;
        Ok(TeamAndItems {
            unknown: unknown,
            team: team,
            orig_pokemon_data: poke_data,
            remaining_data: remaining,
        })
    }
}

impl SectionWrite for TeamAndItems {
    fn id(&self) -> u16 {
        1
    }
    fn cksum_area_len(&self) -> u64 {
        3968
    }
    fn write_data<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.unknown.write(writer)?;
        writer.write_u32::<LE>(self.team.len() as u32)?;
        for pokemon in &self.team {
            pokemon.write(writer)?;
        }
        let empty_slots_left = 6 - self.team.len();
        debug!("{} pokemon written, {} empty slots left",
               self.team.len(),
               empty_slots_left);
        // Fill out rest of pokemon slots with zero bytes
        let offset = self.team.len() * 100;
        writer.write_all(&self.orig_pokemon_data[offset..])?;
        self.remaining_data.write(writer)
    }
}

impl TeamAndItemsUnknown {
    fn read<R: Read>(reader: &mut R, game_type: GameType) -> Result<Self, Box<Error>> {
        Ok(match game_type {
            GameType::Emerald | GameType::RubyOrSapphire => {
                let mut buffer = [0; EM_RU_SA_TEAMANDITEMS_UNK_LEN];
                reader.read_exact(&mut buffer)?;
                TeamAndItemsUnknown::EmeraldOrRubyOrSapphire(buffer)
            }
            GameType::FireredOrLeafgreen => {
                let mut buffer = [0; FR_LG_TEAMANDITEMS_UNK_LEN];
                reader.read_exact(&mut buffer)?;
                TeamAndItemsUnknown::FireRedOrLeafGreen(buffer)
            }
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match *self {
            TeamAndItemsUnknown::EmeraldOrRubyOrSapphire(ref data) => writer.write_all(data),
            TeamAndItemsUnknown::FireRedOrLeafGreen(ref data) => writer.write_all(data),
        }
    }
}

impl TeamAndItemsRemaining {
    fn read<R: Read>(reader: &mut R, game_type: GameType) -> io::Result<Self> {
        Ok(match game_type {
            GameType::Emerald | GameType::RubyOrSapphire => {
                let mut buf = [0; EM_RU_SA_TEAMANDITEMS_REM_LEN];
                reader.read_exact(&mut buf)?;
                TeamAndItemsRemaining::EmeraldOrRubyOrSapphire(buf)
            }
            GameType::FireredOrLeafgreen => {
                let mut buf = [0; FR_LG_TEAMANDITEMS_REM_LEN];
                reader.read_exact(&mut buf)?;
                TeamAndItemsRemaining::FireredOrLeafgreen(buf)
            }
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match *self {
            TeamAndItemsRemaining::EmeraldOrRubyOrSapphire(ref data) => writer.write_all(data),
            TeamAndItemsRemaining::FireredOrLeafgreen(ref data) => writer.write_all(data),
        }
    }
}

impl Pokemon {
    fn read_non_active<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        let personality_value = reader.read_u32::<LE>()?;
        let ot_id = reader.read_u32::<LE>()?;
        let mut nick = [0; POKEMON_NICK_LEN];
        reader.read_exact(&mut nick)?;
        let language = reader.read_u16::<LE>()?;
        let mut ot_name = [0; TRAINER_NAME_LEN];
        reader.read_exact(&mut ot_name)?;
        let markings = reader.read_u8()?;
        let checksum = reader.read_u16::<LE>()?;
        let unknown_1 = reader.read_u16::<LE>()?;
        debug!("Pokemon with pv {}, otid {}, nick {}, otnick {}",
               personality_value,
               ot_id,
               rgen3_string::decode_string(&nick),
               rgen3_string::decode_string(&ot_name));
        let data = PokemonData::read(reader, personality_value, ot_id)?;
        Ok(Pokemon {
            personality: personality_value,
            ot_id: ot_id,
            nickname: PokemonNick(nick),
            language: language,
            ot_name: TrainerName(ot_name),
            markings: markings,
            checksum: checksum,
            unknown_1: unknown_1,
            data: data,
            active_data: None,
        })
    }
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        let mut pokemon = Self::read_non_active(reader)?;
        pokemon.active_data = Some(PokemonActiveData::read(reader)?);
        Ok(pokemon)
    }
    fn write_non_active<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        debug!("Writing data for Pokemon {:?}", self.nickname);
        writer.write_u32::<LE>(self.personality)?;
        writer.write_u32::<LE>(self.ot_id)?;
        writer.write_all(&self.nickname.0)?;
        writer.write_u16::<LE>(self.language)?;
        writer.write_all(&self.ot_name.0)?;
        writer.write_u8(self.markings)?;
        let mut data_buf = [0u8; POKEMON_DATA_LEN];
        {
            let mut writer = &mut data_buf[..];
            self.data.write_unencrypted(&mut writer, self.personality)?;
        }
        let cksum = Pokemon::calc_data_checksum(&data_buf)?;
        debug!("Calculated checksum for data section is {}", cksum);
        writer.write_u16::<LE>(cksum)?;
        debug!("Writing unknown 1 of value {}", self.unknown_1);
        writer.write_u16::<LE>(self.unknown_1)?;
        self.encrypt_data(&mut data_buf);
        writer.write_all(&data_buf)
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.write_non_active(writer)?;
        let active_data = self.active_data
            .as_ref()
            .expect("Requested full write on pokemon not having active data");
        active_data.write(writer)
    }
    fn calc_data_checksum(mut data: &[u8]) -> io::Result<u16> {
        let mut accum: u16 = 0;
        for _ in 0..POKEMON_DATA_LEN / 2 {
            accum = accum.wrapping_add(data.read_u16::<LE>()?);
        }
        Ok(accum)
    }
    fn encrypt_data(&self, data: &mut [u8; POKEMON_DATA_LEN]) {
        debug!("Unencrypted data when writing is {:?}", &data[..]);
        let mut encrypted = [0; POKEMON_DATA_LEN];
        let encryption_key = self.ot_id ^ self.personality;
        {
            let mut reader = &data[..];
            let mut writer = &mut encrypted[..];
            for _ in 0..POKEMON_DATA_LEN / 4 {
                let value = reader.read_u32::<LE>().unwrap();
                writer.write_u32::<LE>(value ^ encryption_key).unwrap();
            }
        }
        *data = encrypted;
        debug!("Encrypted data when writing is {:?}", &data[..]);
    }
}

const POKEMON_DATA_LEN: usize = 48;

// Read/Write the PokéData sections in different order depending on the order value
macro_rules! rw_order {
    ($rw:ident, $order:expr) => {
        match $order {
            0 => $rw!(G A E M),
            1 => $rw!(G A M E),
            2 => $rw!(G E A M),
            3 => $rw!(G E M A),
            4 => $rw!(G M A E),
            5 => $rw!(G M E A),
            6 => $rw!(A G E M),
            7 => $rw!(A G M E),
            8 => $rw!(A E G M),
            9 => $rw!(A E M G),
            10 => $rw!(A M G E),
            11 => $rw!(A M E G),
            12 => $rw!(E G A M),
            13 => $rw!(E G M A),
            14 => $rw!(E A G M),
            15 => $rw!(E A M G),
            16 => $rw!(E M G A),
            17 => $rw!(E M A G),
            18 => $rw!(M G A E),
            19 => $rw!(M G E A),
            20 => $rw!(M A G E),
            21 => $rw!(M A E G),
            22 => $rw!(M E G A),
            23 => $rw!(M E A G),
            _ => unreachable!(),
        }
    }
}

impl PokemonData {
    fn read<R: Read>(reader: &mut R, pv: u32, ot_id: u32) -> Result<Self, Box<Error>> {
        macro_rules! r {
            ($r1:ident $r2:ident $r3:ident $r4:ident) => {{
                let (growth, attacks, evs_and_condition, misc);
                let dk = ot_id ^ pv;
                macro_rules! read_section {
                    (G) => {growth = PokemonGrowth::read(reader, dk)?};
                    (A) => {attacks = PokemonAttacks::read(reader, dk)?};
                    (E) => {evs_and_condition = PokemonEvsAndCondition::read(reader, dk)?};
                    (M) => {misc = PokemonMisc::read(reader, dk)?};
                }
                read_section!($r1);
                read_section!($r2);
                read_section!($r3);
                read_section!($r4);

                Ok(PokemonData {
                    growth: growth,
                    attacks: attacks,
                    evs_and_condition: evs_and_condition,
                    misc: misc,
                })
            }};
        }
        let order = pv % 24;
        debug!("Section read order is {}", order);
        rw_order!(r, order)
    }
    fn write_unencrypted<W: Write>(&self, writer: &mut W, pv: u32) -> io::Result<()> {
        macro_rules! w {
            ($w1:ident $w2:ident $w3:ident $w4:ident) => {{
                macro_rules! write_section {
                    (G) => {self.growth.write_unencrypted(writer)?};
                    (A) => {self.attacks.write_unencrypted(writer)?};
                    (E) => {self.evs_and_condition.write_unencrypted(writer)?};
                    (M) => {self.misc.write_unencrypted(writer)?};
                }
                write_section!($w1);
                write_section!($w2);
                write_section!($w3);
                write_section!($w4);
            }};
        }
        let order = pv % 24;
        debug!("Section write order is {}", order);
        rw_order!(w, order);
        Ok(())
    }
}

const SUBSTRUCTURE_LEN: usize = 12;

fn read_and_decrypt<R: Read>(reader: &mut R,
                             dec_key: u32)
                             -> Result<[u8; SUBSTRUCTURE_LEN], Box<Error>> {
    let mut data = [0; SUBSTRUCTURE_LEN];
    reader.read_exact(&mut data)?;
    debug!("Encrypted data when reading substructure: {:?}", &data[..]);
    let (n1, n2, n3);
    {
        let mut reader = &data[..];
        n1 = reader.read_u32::<LE>()?;
        n2 = reader.read_u32::<LE>()?;
        n3 = reader.read_u32::<LE>()?;
    }
    {
        let mut writer = &mut data[..];
        writer.write_u32::<LE>(n1 ^ dec_key)?;
        writer.write_u32::<LE>(n2 ^ dec_key)?;
        writer.write_u32::<LE>(n3 ^ dec_key)?;
    }
    debug!("Unencrypted data when reading substructure: {:?}",
           &data[..]);
    Ok(data)
}

impl PokemonGrowth {
    fn read<R: Read>(reader: &mut R, dec_key: u32) -> Result<Self, Box<Error>> {
        let data = read_and_decrypt(reader, dec_key)?;
        let mut reader = &data[..];
        Ok(PokemonGrowth {
            species: reader.read_u16::<LE>()?,
            item_held: reader.read_u16::<LE>()?,
            experience: reader.read_u32::<LE>()?,
            pp_bonuses: reader.read_u8()?,
            friendship: reader.read_u8()?,
            unknown: reader.read_u16::<LE>()?,
        })
    }
    fn write_unencrypted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LE>(self.species)?;
        writer.write_u16::<LE>(self.item_held)?;
        writer.write_u32::<LE>(self.experience)?;
        writer.write_u8(self.pp_bonuses)?;
        writer.write_u8(self.friendship)?;
        writer.write_u16::<LE>(self.unknown)
    }
}

impl PokemonAttacks {
    fn read<R: Read>(reader: &mut R, dec_key: u32) -> Result<Self, Box<Error>> {
        let data = read_and_decrypt(reader, dec_key)?;
        let mut reader = &data[..];
        Ok(PokemonAttacks {
            move1: reader.read_u16::<LE>()?,
            move2: reader.read_u16::<LE>()?,
            move3: reader.read_u16::<LE>()?,
            move4: reader.read_u16::<LE>()?,
            pp1: reader.read_u8()?,
            pp2: reader.read_u8()?,
            pp3: reader.read_u8()?,
            pp4: reader.read_u8()?,
        })
    }
    fn write_unencrypted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u16::<LE>(self.move1)?;
        writer.write_u16::<LE>(self.move2)?;
        writer.write_u16::<LE>(self.move3)?;
        writer.write_u16::<LE>(self.move4)?;
        writer.write_u8(self.pp1)?;
        writer.write_u8(self.pp2)?;
        writer.write_u8(self.pp3)?;
        writer.write_u8(self.pp4)
    }
}

impl PokemonEvsAndCondition {
    fn read<R: Read>(reader: &mut R, dec_key: u32) -> Result<Self, Box<Error>> {
        let data = read_and_decrypt(reader, dec_key)?;
        let mut reader = &data[..];
        Ok(PokemonEvsAndCondition {
            hp_ev: reader.read_u8()?,
            attack_ev: reader.read_u8()?,
            defense_ev: reader.read_u8()?,
            speed_ev: reader.read_u8()?,
            special_attack_ev: reader.read_u8()?,
            special_defense_ev: reader.read_u8()?,
            coolness: reader.read_u8()?,
            beauty: reader.read_u8()?,
            cuteness: reader.read_u8()?,
            smartness: reader.read_u8()?,
            toughness: reader.read_u8()?,
            feel: reader.read_u8()?,
        })
    }
    fn write_unencrypted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self.hp_ev)?;
        writer.write_u8(self.attack_ev)?;
        writer.write_u8(self.defense_ev)?;
        writer.write_u8(self.speed_ev)?;
        writer.write_u8(self.special_attack_ev)?;
        writer.write_u8(self.special_defense_ev)?;
        writer.write_u8(self.coolness)?;
        writer.write_u8(self.beauty)?;
        writer.write_u8(self.cuteness)?;
        writer.write_u8(self.smartness)?;
        writer.write_u8(self.toughness)?;
        writer.write_u8(self.feel)
    }
}

impl PokemonMisc {
    fn read<R: Read>(reader: &mut R, dec_key: u32) -> Result<Self, Box<Error>> {
        let data = read_and_decrypt(reader, dec_key)?;
        let mut reader = &data[..];
        Ok(PokemonMisc {
            pokerus_status: reader.read_u8()?,
            met_location: reader.read_u8()?,
            origins_info: reader.read_u16::<LE>()?,
            ivs_eggs_and_ability: reader.read_u32::<LE>()?,
            ribbons_and_obedience: reader.read_u32::<LE>()?,
        })
    }
    fn write_unencrypted<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self.pokerus_status)?;
        writer.write_u8(self.met_location)?;
        writer.write_u16::<LE>(self.origins_info)?;
        writer.write_u32::<LE>(self.ivs_eggs_and_ability)?;
        writer.write_u32::<LE>(self.ribbons_and_obedience)
    }
}

impl PokemonActiveData {
    fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        Ok(PokemonActiveData {
            status_condition: reader.read_u32::<LE>()?,
            level: reader.read_u8()?,
            pokerus_remaining: reader.read_u8()?,
            current_hp: reader.read_u16::<LE>()?,
            total_hp: reader.read_u16::<LE>()?,
            attack: reader.read_u16::<LE>()?,
            defense: reader.read_u16::<LE>()?,
            speed: reader.read_u16::<LE>()?,
            sp_attack: reader.read_u16::<LE>()?,
            sp_defense: reader.read_u16::<LE>()?,
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32::<LE>(self.status_condition)?;
        writer.write_u8(self.level)?;
        writer.write_u8(self.pokerus_remaining)?;
        writer.write_u16::<LE>(self.current_hp)?;
        writer.write_u16::<LE>(self.total_hp)?;
        writer.write_u16::<LE>(self.attack)?;
        writer.write_u16::<LE>(self.defense)?;
        writer.write_u16::<LE>(self.speed)?;
        writer.write_u16::<LE>(self.sp_attack)?;
        writer.write_u16::<LE>(self.sp_defense)
    }
}

impl PcBuffer {
    fn read<R: Read>(reader: &mut R, index: usize) -> Result<Self, Box<Error>> {
        let mut data = [0u8; DATA_SIZE as usize];
        reader.read_exact(&mut data)?;
        Ok(PcBuffer {
            data: data,
            index: index,
        })
    }
}

impl SectionWrite for PcBuffer {
    fn id(&self) -> u16 {
        self.index as u16 + 5
    }
    fn cksum_area_len(&self) -> u64 {
        if self.id() == 13 { 2000 } else { 3968 }
    }
    fn write_data<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.data)
    }
}

impl PokemonStorage {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        let current_box = reader.read_u32::<LE>()?;
        debug!("Current box: {}", current_box);
        let mut boxes = [PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?,
                         PokeBox::read(reader)?];
        for b in &mut boxes {
            reader.read_exact(&mut b.name.0)?;
        }
        for b in &mut boxes {
            b.wallpaper = reader.read_u8()?;
        }
        Ok(PokemonStorage {
            current_box: current_box as usize,
            boxes: boxes,
        })
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u32::<LE>(self.current_box as u32)?;
        for b in &self.boxes {
            b.write(writer)?;
        }
        for b in &self.boxes {
            writer.write_all(&b.name.0)?;
        }
        for b in &self.boxes {
            writer.write_u8(b.wallpaper)?;
        }
        Ok(())
    }
}

impl PokeBox {
    fn read<R: Read>(reader: &mut R) -> Result<Self, Box<Error>> {
        let mut poke_box = PokeBox::default();
        for opt_pokemon in &mut poke_box.pokemon {
            let mut data = [0; 80];
            reader.read_exact(&mut data)?;
            // If the entire data is zero bytes, then the slot is empty
            if !data.iter().all(|&v| v == 0) {
                *opt_pokemon = Some(Pokemon::read_non_active(&mut &data[..])?);
            }
        }
        Ok(poke_box)
    }
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for opt_pokemon in &self.pokemon {
            match *opt_pokemon {
                Some(ref pokemon) => pokemon.write_non_active(writer)?,
                None => writer.write_all(&[0; 80])?,
            }
        }
        Ok(())
    }
}
