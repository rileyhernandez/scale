# Scale

A Rust library for interfacing with Phidget-based load cells and scales, providing weight measurement, calibration, and action detection capabilities for food service applications.

## Overview

This library provides a high-level interface for working with Phidget voltage ratio input devices connected to load cells. It includes features for:

- **Real-time weight measurement** with stability detection
- **Automatic action detection** (serving, refilling) based on weight changes  
- **Scale calibration** with gain and offset calculations
- **Phidget device discovery** and connection management
- **Configurable noise filtering** and stability thresholds

## Architecture

### Core Components

- **`DisconnectedScale`** - Configuration and device management before connection
- **`Scale`** - Connected scale instance with measurement capabilities
- **`Weight`** - Enum representing stable or unstable weight readings
- **`Action`** - Enum for detected scale actions (Served, Refilled, etc.)
- **`Error`** - Comprehensive error handling for all operations

### Key Features

**Weight Measurement:**
- Raw voltage ratio readings from Phidget devices
- Calibrated weight calculations using gain/offset
- Stability detection via configurable noise thresholds
- Buffered readings for improved accuracy

**Action Detection:**
- Automatic detection of serving and refilling events
- Configurable sensitivity thresholds
- Delta tracking between stable weight measurements

**Device Management:**
- USB device discovery for connected Phidgets
- Serial number-based device identification
- Connection lifecycle management

## Dependencies

- **phidget** - Core Phidget device interface
- **menu** - Configuration and device management (external git dependency)
- **thiserror** - Error handling
- **reqwest** - HTTP client functionality
- **serde_json** - JSON serialization
- **time** - Time handling utilities
- **log** - Logging framework
- **rusb** - USB device discovery (optional feature)

## Features

- `find_phidgets` - Enables USB device discovery functionality

## Usage

### Basic Scale Connection

```rust
use scale::{DisconnectedScale, Config, Device};
use menu::device::Model;

// Create scale configuration
let config = Config {
    phidget_id: 716588,
    load_cell_id: 0,
    gain: 10000.0,
    offset: 0.0,
    // ... other config options
};

// Create and connect scale
let device = Device::new(Model::LibraV0, "L0");
let disconnected_scale = DisconnectedScale::new(config, device);
let mut scale = disconnected_scale.connect()?;
```

### Weight Measurement

```rust
// Get current weight reading
let weight = scale.get_weight()?;
match weight {
    Weight::Stable(w) => println!("Stable weight: {:.1}g", w),
    Weight::Unstable(w) => println!("Unstable weight: {:.1}g", w),
}

// Wait for settled reading
let settled_weight = scale.weigh_once_settled(
    3,                           // stable samples required
    Duration::from_secs(10),     // timeout
    0.1                          // max noise ratio
)?;
```

### Action Detection

```rust
// Check for scale actions (serving, refilling)
if let Some((action, delta)) = scale.check_for_action() {
    println!("Action detected: {} (Δ{:.1}g)", action, delta);
}
```

### Calibration

```rust
// Calibrate scale with known weights
let empty_reading = -0.000003141;  // Raw reading with no weight
let weight_reading = 0.0001232;    // Raw reading with known weight
let known_weight = 1277.0;         // Known calibration weight in grams

scale.set_calibration(empty_reading, weight_reading, known_weight);
```

### Configuration from File

```rust
// Load multiple scale configurations from file
let scales = DisconnectedScale::from_config(Path::new("scales.json"))?;
for scale_config in scales {
    let scale = scale_config.connect()?;
    // Use scale...
}
```

## Configuration

Scale behavior is controlled via the `Config` struct with parameters for:

- **Phidget connection** - Device ID, channel, sample rate
- **Calibration** - Gain and offset values  
- **Stability detection** - Buffer size, noise thresholds
- **Action sensitivity** - Delta thresholds for event detection

## Error Handling

The library provides comprehensive error handling for:

- Phidget device communication errors
- USB connection issues
- Network/HTTP errors
- Configuration parsing errors
- Timeout conditions

## Testing

The library includes integration tests that can be run with an actual Phidget device connected:

```bash
cargo test
```

Note: Tests require a properly configured Phidget device to pass.

## Internal Architecture Notes

- Uses a rolling buffer system for stability detection
- Implements automatic reconnection capabilities
- Supports concurrent access to multiple scales
- Optimized for real-time food service applications
- Integrates with external menu management system
