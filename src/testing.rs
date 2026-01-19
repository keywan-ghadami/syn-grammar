use std::fmt::Debug;

/// Ein Wrapper um syn::Result, um flüssige Tests zu schreiben.
pub struct TestResult<T> {
    inner: syn::Result<T>,
}

impl<T: Debug> TestResult<T> {
    pub fn new(result: syn::Result<T>) -> Self {
        Self { inner: result }
    }

    /// 1. Behauptet Erfolg und gibt den Wert zurück (wie bisher).
    pub fn assert_success(self) -> T {
        match self.inner {
            Ok(val) => val,
            Err(e) => {
                panic!(
                    "\n🔴 TEST FAILED (Expected Success, but got Error):\nMessage:  {}\nLocation: {:?}\n", 
                    e, e.span()
                );
            }
        }
    }

    /// 2. NEU: Behauptet Erfolg UND prüft direkt den Wert.
    /// Gibt eine schöne Diff-Ausgabe, wenn die Werte nicht übereinstimmen.
    pub fn assert_success_is<E>(self, expected: E) -> T 
    where T: PartialEq<E>, E: Debug {
        let val = self.assert_success();
        if val != expected {
            panic!(
                "\n🔴 TEST FAILED (Value Mismatch):\nExpected: {:?}\nGot:      {:?}\n", 
                expected, val
            );
        }
        val
    }

    /// 3. Behauptet Fehler und gibt den Error zurück.
    pub fn assert_failure(self) -> syn::Error {
        match self.inner {
            Ok(val) => {
                panic!(
                    "\n🔴 TEST FAILED (Expected Failure, but got Success):\nParsed Value: {:?}\n", 
                    val
                );
            }
            Err(e) => e,
        }
    }

    /// 4. NEU: Behauptet Fehler UND prüft, ob die Meldung einen Text enthält.
    pub fn assert_failure_contains(self, expected_msg_part: &str) {
        let err = self.assert_failure();
        let actual_msg = err.to_string();
        if !actual_msg.contains(expected_msg_part) {
            panic!(
                "\n🔴 TEST FAILED (Error Message Mismatch):\nExpected part: {:?}\nActual msg:    {:?}\nLocation:      {:?}\n", 
                expected_msg_part, actual_msg, err.span()
            );
        }
    }
}

pub trait Testable<T> {
    fn test(self) -> TestResult<T>;
}

impl<T: Debug> Testable<T> for syn::Result<T> {
    fn test(self) -> TestResult<T> {
        TestResult::new(self)
    }
}
