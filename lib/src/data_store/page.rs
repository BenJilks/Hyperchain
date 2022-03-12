/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CreatePageData
{
    pub name: String,
    pub page: Vec<u8>,
}

impl CreatePageData
{

    pub fn new(name: String, page: Vec<u8>) -> Self
    {
        Self
        {
            name,
            page,
        }
    }

}

