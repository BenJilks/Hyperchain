/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct ErrorMessage
{
    message: String,
}

impl ErrorMessage
{

    pub fn new(message: &str) -> Box<Self>
    {
        Box::from(Self
        {
            message: message.to_owned(),
        })
    }

}

impl Display for ErrorMessage
{

    fn fmt(&self, f: &mut Formatter) 
        -> Result<(), std::fmt::Error>
    {
        write!(f, "Error: {}", self.message)
    }

}

impl std::error::Error for ErrorMessage
{
}

