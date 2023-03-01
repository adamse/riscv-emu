
#[derive(Debug)]
pub enum Error {
    OutOfBounds,
    NoFit,
}

/// Set of ranges
#[derive(Debug)]
pub struct RangeSet {
    /// ranges that are in the set
    /// (inclusive, exclusive)
    ranges: Vec<(u32, u32)>,
}

impl RangeSet {
    /// Allocate a new range set with the initial range
    pub fn new(start: u32, end: u32) -> Self {
        RangeSet {
            ranges: vec![(start, end)],
        }
    }

    /// Remove a range from the set, the range to remove must be contigous in the set
    ///
    pub fn remove(&mut self, start: u32, end: u32) -> Result<(), Error> {
        // find range currently in the set which includes the one we want to remove
        // this means that
        // - start is after the range start and before the range end and
        // - end is before the range end

        let range = self.ranges.iter_mut().enumerate()
            .find(|(_, &mut range)|
                start >= range.0 &&
                start < range.1 &&
                end <= range.1);

        let Some((ii, range)) = range else {
            // TODO: better error
            return Err(Error::OutOfBounds);
        };

        if start == range.0 && end == range.1 {
            // is it the whole range?
            self.ranges.remove(ii);
        } else if start == range.0 {
            // is our range at the start of the found range?
            // if so just truncate the range
            range.0 = end;
        } else if end == range.1 {
            range.1 = start;
        } else {
            // we need to split the range
            let r1 = (range.0, start);
            let r2 = (end, range.1);
            self.ranges[ii] = r1;
            self.ranges.insert(ii + 1, r2);
        }

        Ok(())
    }

    /// Remove a range that is `size` big using a first fit strategy.
    ///
    pub fn remove_first_fit(&mut self, size: u32) -> Result<(u32, u32), Error> {
        let fit = self.ranges.iter().find(|range| size <= range.1 - range.0);

        let Some(&(start, _)) = fit else {
            return Err(Error::NoFit);
        };

        self.remove(start, start + size)?;

        Ok((start, start + size))
    }

    /// Insert a range into the set
    ///
    pub fn insert(&mut self, start: u32, end: u32) -> Result<(), Error> {
        // find the place to insert the range
        //
        // New range cases:
        // - a. end is before the start
        //     new: <   >
        //     set:       <    >    <   > ...
        // - b. range is after all other ranges
        //     new:                 <   >
        //     set: ... <    > <   >
        //
        // Merge cases:
        // - c. start before start, end after start
        //     new: <   >
        //     set:     <    >    <   > ...
        //
        // - d. start is before the end
        //     new:      <   >
        //     set: <    > ...
        //
        // - e. new spans many ranges
        //     new: <                 >
        //     set:    < >   < >    <    >

        /*
        let place = self.ranges.iter_mut().enumerate()
            .find(|(ii, &mut range)|
                // case a and c
                end <= range.0 ||
                // case d
                start <= range.1);
        */

        todo!("implement this :)");
    }
}

mod test {

    #[test]
    fn rangeset() {
        use super::*;

        let mut rs = RangeSet::new(0, 1024);
        println!("{rs:?}");
        assert!(rs.remove(0, 512).is_ok());
        println!("{rs:?}");

        let mut rs = RangeSet::new(0, 1024);
        println!("{rs:?}");
        assert!(rs.remove(1, 512).is_ok());
        println!("{rs:?}");

        let mut rs = RangeSet::new(0, 1024);
        println!("{rs:?}");
        assert!(rs.remove(512, 1024).is_ok());
        println!("{rs:?}");

        let mut rs = RangeSet::new(0, 1024);
        println!("{rs:?}");
        assert!(rs.remove(512, 1025).is_err());
        println!("{rs:?}");

        let mut rs = RangeSet::new(0, 1024);
        println!("{rs:?}");
        assert!(rs.remove_first_fit(512).is_ok());
        println!("{rs:?}");
        assert!(rs.remove_first_fit(12).is_ok());
        println!("{rs:?}");
    }
}
