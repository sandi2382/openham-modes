# On-Air Preamble Recommendations

This document describes recommended practices for identifying experimental transmissions using OpenHam digital modes.

## Disclaimer

**THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND.** Users are solely responsible for ensuring compliance with local amateur radio regulations, licensing requirements, and band plans. Use at your own risk.

## Legal Requirements

Before transmitting with experimental digital modes:

1. **Follow local amateur radio regulations** - Ensure you are licensed and operating within band plans
2. **Respect power limits** - Use appropriate power levels for your license class and band
3. **Avoid encryption** - All OpenHam modes are designed to be non-encrypted
4. **Identify properly** - Follow identification requirements for your country

## Recommended Preamble Sequence

### 1. CW Identification
Send your callsign in Morse code at the beginning of each transmission session:
```
DE [YOUR_CALLSIGN] [YOUR_CALLSIGN] K
```

### 2. Spoken Identification
Follow with a clear voice announcement:
```
"This is [YOUR_CALLSIGN] conducting experimental transmissions 
using OpenHam digital modes. Specifications available at 
github.com/sandi2382/openham-modes"
```

### 3. Optional: Mode Information
For uncommon modes, consider adding:
```
"Now transmitting [MODE_NAME] as specified in [SPEC_VERSION]"
```

## Example Complete Preamble

```
[CW] DE S56SPZ S56SPZ K
[VOICE] "This is S56SPZ conducting experimental transmissions using 
OpenHam digital modes, specification published at 
github.com/sandi2382/openham-modes. Now transmitting 
OpenHam Text Mode version 1.0"
[DIGITAL] [Mode-specific synchronization and data follows]
```

## Timing Recommendations

- **CW ID**: 15-20 WPM for clarity
- **Voice ID**: Clear, moderate pace (avoid rushing)
- **Total preamble time**: Keep under 30 seconds when possible
- **Repeat interval**: Re-identify every 10 minutes maximum (follow local rules)

## Band-Specific Considerations

### HF Bands
- Use appropriate sub-bands for digital modes
- Consider propagation conditions
- Be aware of regional band plans

### VHF/UHF Bands
- Coordinate with local repeater groups
- Use designated digital frequencies when available
- Consider interference to other services

## Special Situations

### Weak Signal Work
For very weak signal modes:
- Send CW ID at higher power if possible
- Use standard phonetics for voice ID
- Consider beacon-style identification

### High-Speed Modes
For rapid data transmission:
- Send ID before and after data bursts
- Include station ID in digital stream when possible
- Use error correction for ID portions

### Automated Stations
- Include callsign in every frame header
- Periodic voice announcements (if unattended operation allowed)
- Remote shutdown capability

## Technical Implementation

### In Mode Specifications
Each mode specification should include:
- Required preamble patterns
- Sync word recommendations  
- Station identification methods
- Error handling for ID failures

### In Software
- Automated preamble generation
- Timer-based re-identification
- User-configurable callsign storage
- Compliance checking features

## International Considerations

When operating in different countries:
- Research local amateur regulations
- Use appropriate language for voice ID
- Include location information if required
- Respect local band plans and etiquette

## Emergency Communications

During emergency communications:
- Follow established protocols
- Prioritize essential traffic
- Maintain identification requirements
- Coordinate with served agencies

## References

- [Your Country's Amateur Radio Regulations]
- [Local Band Plan]
- [ITU Radio Regulations]
- [OpenHam Mode Specifications](../specs/)

---

**Note**: This document provides general guidance. Always consult your local amateur radio authority for specific requirements in your jurisdiction.