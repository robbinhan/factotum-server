// Copyright (c) 2017-2018 Snowplow Analytics Ltd. All rights reserved.
//
// This program is licensed to you under the Apache License Version 2.0, and
// you may not use this file except in compliance with the Apache License
// Version 2.0.  You may obtain a copy of the Apache License Version 2.0 at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the Apache License Version 2.0 is distributed on an "AS
// IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
// implied.  See the Apache License Version 2.0 for the specific language
// governing permissions and limitations there under.
//

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
struct GoodPersistenceMock {
    id: String,
    ref_map: RefCell<HashMap<String, String>>,
}

impl GoodPersistenceMock {
    fn new(id: &str) -> Self {
        GoodPersistenceMock {
            id: id.to_owned(),
            ref_map: RefCell::new(HashMap::new()),
        }
    }
}

impl Persistence for GoodPersistenceMock {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_key(&self, key: &str, value: &str) -> ThreadResult<()> {
        let mut map = self.ref_map.borrow_mut();
        map.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn get_key(&self, key: &str) -> ThreadResult<Option<String>> {
        let map = self.ref_map.borrow();
        let value = map.get(key);
        Ok(value.map(|s| s.to_owned()))
    }

    fn prepend_namespace(&self, key: &str) -> String {
        apply_namespace_if_absent("com.test/namespace", key)
    }
}

#[derive(Debug)]
struct BadPersistenceMock;

impl Persistence for BadPersistenceMock {
    fn id(&self) -> &str {
        "something_bad"
    }

    fn set_key(&self, _: &str, _: &str) -> ThreadResult<()> {
        Err(Box::new("setting key bad"))
    }

    fn get_key(&self, _: &str) -> ThreadResult<Option<String>> {
        Err(Box::new("getting key bad"))
    }

    fn prepend_namespace(&self, key: &str) -> String {
        key.to_string()
    }
}

#[test]
fn set_entry_fail_error() {
    let persistence = BadPersistenceMock;
    let request = JobRequest::new("", "dummy", "/fake/path", vec![]);

    let result = set_entry(&persistence, "fake_entry", &request, &JobState::QUEUED, &JobOutcome::WAITING);

    assert_eq!(false, result);
}

#[test]
fn set_entry_new_success() {
    let persistence = GoodPersistenceMock::new("test_set");
    let request = JobRequest::new("", "dummy", "/fake/path", vec![]);

    let result = set_entry(&persistence, "fake_entry", &request, &JobState::QUEUED, &JobOutcome::WAITING);

    let borrowed = &persistence.ref_map.borrow();
    let entry = borrowed.get("com.test/namespace/fake_entry").unwrap();
    let job_entry: JobEntry = serde_json::from_str(entry).expect("JSON decode error");

    assert_eq!(true, result);
    assert_eq!(JobState::QUEUED, job_entry.state);
    assert_eq!("test_set".to_string(), job_entry.last_run_from);
    assert_eq!(JobOutcome::WAITING, job_entry.last_outcome);
    assert_eq!(request, job_entry.job_request);
}

#[test]
fn get_entry_fail_none() {
    let persistence = BadPersistenceMock;

    let result = get_entry(&persistence, "fake_entry");

    assert_eq!(None, result);
}

#[test]
fn get_entry_without_namespace_success_key() {
    use base64::encode;

    let persistence = GoodPersistenceMock::new("test_get");
    let request = JobRequest::new("", "dummy", "/fake/path", vec![]);
    let job_entry = JobEntry::new(&JobState::QUEUED, &request, &persistence.id(), &JobOutcome::WAITING);
    let job_entry_json = serde_json::to_string(&job_entry).expect("JSON compact encode error");
    let encoded_entry = encode(job_entry_json.as_bytes());
    {
        let mut map = persistence.ref_map.borrow_mut();
        map.insert("com.test/namespace/dummy_entry".to_string(), encoded_entry);
    }

    let id = "dummy_entry";
    let result = get_entry(&persistence, id).expect(&format!("Unable to find entry in test persistence! id='{}'", id));

    assert_eq!(JobState::QUEUED, result.state);
    assert_eq!("test_get".to_string(), result.last_run_from);
    assert_eq!(request, result.job_request);
}

#[test]
fn get_entry_with_namespace_success_key() {
    use base64::encode;

    let persistence = GoodPersistenceMock::new("test_get");
    let request = JobRequest::new("", "dummy", "/fake/path", vec![]);
    let job_entry = JobEntry::new(&JobState::QUEUED, &request, &persistence.id(), &JobOutcome::WAITING);
    let job_entry_json = serde_json::to_string(&job_entry).expect("JSON compact encode error");
    let encoded_entry = encode(job_entry_json.as_bytes());
    {
        let mut map = persistence.ref_map.borrow_mut();
        map.insert("com.test/namespace/dummy_entry".to_string(), encoded_entry);
    }

    let id = "com.test/namespace/dummy_entry";
    let result = get_entry(&persistence, id).expect(&format!("Unable to find entry in test persistence! id='{}'", id));

    assert_eq!(JobState::QUEUED, result.state);
    assert_eq!("test_get".to_string(), result.last_run_from);
    assert_eq!(JobOutcome::WAITING, result.last_outcome);
    assert_eq!(request, result.job_request);
}
