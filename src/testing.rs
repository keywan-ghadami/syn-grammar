use std::fmt::Debug;

// Ein Wrapper um syn::Result, um flüssige Tests zu schreiben.
pub struct TestResult<T> {
    inner: syn::Result<T>,
    context: Option<String>,
}

impl<T: Debug> TestResult<T> {
    pub fn new(result: syn::Result<T>) -> Self {
        Self { 
            inner: result,
            context: None,
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }

    fn format_context(&self) -> String {
        self.context.as_ref()
            .map(|c| format!("\nContext:  {}", c))
            .unwrap_or_default()
    }

    // 1. Behauptet Erfolg und gibt den Wert zurück (wie bisher).
    pub fn assert_success(self) -> T {
        match self.inner {
            Ok(val) => val,
            Err(e) => {
                panic!(
                    "\n🔴 TEST FAILED (Expected Success, but got Error):{}\nMessage:  {}\nLocation: {:?}\n", 
                    self.format_context(), e, e.span()
                );
            }
        }
    }

    // 2. NEU: Behauptet Erfolg UND prüft direkt den Wert.
    // Gibt eine schöne Diff-Ausgabe, wenn die Werte nicht übereinstimmen.
    pub fn assert_success_is<E>(self, expected: E) -> T 
    where T: PartialEq<E>, E: Debug {
        let ctx = self.format_context();
        let val = self.assert_success();
        if val != expected {
            panic!(
                "\n🔴 TEST FAILED (Value Mismatch):{}\nExpected: {:?}\nGot:      {:?}\n", 
                ctx, expected, val
            );
        }
        val
    }

    // 3. Behauptet Fehler und gibt den Error zurück.
    pub fn assert_failure(self) -> syn::Error {
        match self.inner {
            Ok(val) => {
                panic!(
                    "\n🔴 TEST FAILED (Expected Failure, but got Success):{}\nParsed Value: {:?}\n", 
                    self.format_context(), val
                );
            }
            Err(e) => e,
        }
    }

    // 4. Behauptet Fehler UND prüft, ob die Meldung einen Text enthält.
    pub fn assert_failure_contains(self, expected_msg_part: &str) {
        let ctx = self.format_context();
        let err = self.assert_failure();
        let actual_msg = err.to_string();
        if !actual_msg.contains(expected_msg_part) {
            panic!(
                "\n🔴 TEST FAILED (Error Message Mismatch):{}\nExpected part: {:?}\nActual msg:    {:?}\nLocation:      {:?}\n", 
                ctx, expected_msg_part, actual_msg, err.span()
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
