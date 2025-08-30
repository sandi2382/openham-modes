//! Python bindings for OpenHam digital modes

use pyo3::prelude::*;
use openham_core::prelude::*;
use openham_frame::prelude::*;
use openham_codecs::prelude::*;
use openham_modem::prelude::*;

/// Python wrapper for SampleBuffer
#[pyclass]
struct PySampleBuffer {
    inner: SampleBuffer<f64>,
}

#[pymethods]
impl PySampleBuffer {
    #[new]
    fn new(capacity: usize, sample_rate: f64) -> PyResult<Self> {
        let buffer = SampleBuffer::new(capacity, sample_rate)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(Self { inner: buffer })
    }
    
    fn len(&self) -> usize {
        self.inner.len()
    }
    
    fn sample_rate(&self) -> f64 {
        self.inner.sample_rate()
    }
    
    fn clear(&mut self) {
        self.inner.clear();
    }
}

/// Python wrapper for Huffman text codec
#[pyclass]
struct PyHuffmanCodec {
    inner: openham_codecs::text::HuffmanCodec,
}

#[pymethods]
impl PyHuffmanCodec {
    #[new]
    fn new() -> Self {
        Self {
            inner: openham_codecs::text::HuffmanCodec::new_english(),
        }
    }
    
    fn encode(&mut self, text: &str) -> PyResult<Vec<u8>> {
        self.inner.encode(text)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    
    fn decode(&mut self, data: &[u8]) -> PyResult<String> {
        self.inner.decode(data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    
    fn compression_ratio(&self) -> f64 {
        self.inner.compression_ratio()
    }
    
    fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Python wrapper for Frame
#[pyclass]
struct PyFrame {
    inner: Frame,
}

#[pymethods]
impl PyFrame {
    #[new]
    fn new(frame_type: u8, sequence: u16, payload: Vec<u8>, flags: u8) -> Self {
        Self {
            inner: Frame::new(frame_type, sequence, payload, flags),
        }
    }
    
    fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes()
    }
    
    #[staticmethod]
    fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        let frame = Frame::from_bytes(bytes)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(Self { inner: frame })
    }
    
    fn total_size(&self) -> usize {
        self.inner.total_size()
    }
    
    #[getter]
    fn payload(&self) -> Vec<u8> {
        self.inner.payload.clone()
    }
    
    #[getter]
    fn sequence(&self) -> u16 {
        self.inner.header.sequence
    }
    
    #[getter]
    fn frame_type(&self) -> u8 {
        self.inner.header.frame_type
    }
}

/// Python wrapper for ModulationConfig
#[pyclass]
struct PyModulationConfig {
    inner: ModulationConfig,
}

#[pymethods]
impl PyModulationConfig {
    #[new]
    fn new(sample_rate: f64, symbol_rate: f64, carrier_frequency: f64) -> PyResult<Self> {
        let config = ModulationConfig::new(sample_rate, symbol_rate, carrier_frequency)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        
        Ok(Self { inner: config })
    }
    
    fn samples_per_symbol(&self) -> f64 {
        self.inner.samples_per_symbol()
    }
    
    #[getter]
    fn sample_rate(&self) -> f64 {
        self.inner.sample_rate
    }
    
    #[getter]
    fn symbol_rate(&self) -> f64 {
        self.inner.symbol_rate
    }
    
    #[getter]
    fn carrier_frequency(&self) -> f64 {
        self.inner.carrier_frequency
    }
}

/// OpenHam digital modes Python module
#[pymodule]
fn openham_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySampleBuffer>()?;
    m.add_class::<PyHuffmanCodec>()?;
    m.add_class::<PyFrame>()?;
    m.add_class::<PyModulationConfig>()?;
    
    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // Add frame type constants
    m.add("FRAME_TYPE_DATA", 0u8)?;
    m.add("FRAME_TYPE_CONTROL", 1u8)?;
    m.add("FRAME_TYPE_BEACON", 2u8)?;
    m.add("FRAME_TYPE_ARQ", 3u8)?;
    
    // Add frame flag constants
    m.add("FRAME_FLAG_MORE_FRAGMENTS", 0x01u8)?;
    m.add("FRAME_FLAG_FRAGMENTED", 0x02u8)?;
    m.add("FRAME_FLAG_PRIORITY", 0x04u8)?;
    m.add("FRAME_FLAG_BROADCAST", 0x08u8)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_bindings_compile() {
        // Basic compilation test
        let _config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
    }
}