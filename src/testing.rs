use std::fmt::Debug;

/// Ein Wrapper um syn::Result, um flüssige Tests zu schreiben.
pub struct TestResult<T> {
    inner: syn::Result<T>,
}

impl<T: Debug> TestResult<T> {
    /// Erstellt einen neuen Test-Wrapper
    pub fn new(result: syn::Result<T>) -> Self {
        Self { inner: result }
    }

    /// Behauptet, dass das Parsen erfolgreich war und gibt das Ergebnis zurück.
    /// Bei Fehler panic mit einer schönen Nachricht.
    pub fn assert_success(self) -> T {
        match self.inner {
            Ok(val) => val,
            Err(e) => {
                panic!("\n🔴 TEST FAILED (Expected Success):\nError Message: {}\nLocation: {:?}\n", e, e.span());
            }
        }
    }

    /// Behauptet, dass das Parsen fehlgeschlagen ist.
    /// Gibt den Fehler zurück, falls man die Fehlermeldung prüfen will.
    pub fn assert_failure(self) -> syn::Error {
        match self.inner {
            Ok(val) => {
                panic!("\n🔴 TEST FAILED (Expected Failure):\nBut parsing succeeded with value: {:?}\n", val);
            }
            Err(e) => e,
        }
    }
}

/// Ein kleines Trait, um .test() direkt auf Results aufrufen zu können
pub trait Testable<T> {
    fn test(self) -> TestResult<T>;
}

impl<T: Debug> Testable<T> for syn::Result<T> {
    fn test(self) -> TestResult<T> {
        TestResult::new(self)
    }
}
