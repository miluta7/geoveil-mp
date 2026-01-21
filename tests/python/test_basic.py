"""
Tests for geoveil_mp Python bindings.
"""
import pytest


def test_import():
    """Test that the module can be imported."""
    import geoveil_mp
    assert geoveil_mp is not None


def test_version():
    """Test that version is accessible."""
    import geoveil_mp as gm
    version = gm.version()
    assert version is not None
    assert isinstance(version, str)
    assert len(version) > 0


def test_constants():
    """Test that constants are accessible."""
    import geoveil_mp as gm
    assert gm.SPEED_OF_LIGHT == 299792458.0
    assert gm.GM_WGS84 > 0
    assert gm.EARTH_RADIUS > 0


def test_gnss_system():
    """Test GnssSystem class."""
    import geoveil_mp as gm
    
    # Test GPS
    gps = gm.GnssSystem("G")
    assert gps.name == "GPS"
    assert gps.code == "G"
    
    # Test Galileo
    gal = gm.GnssSystem("E")
    assert gal.name == "Galileo"
    
    # Test GLONASS
    glo = gm.GnssSystem("R")
    assert glo.name == "GLONASS"
    
    # Test BeiDou
    bds = gm.GnssSystem("C")
    assert bds.name == "BeiDou"


def test_satellite():
    """Test Satellite class."""
    import geoveil_mp as gm
    
    sat = gm.Satellite("G01")
    assert sat.prn == 1
    assert sat.system.name == "GPS"
    assert sat.id == "G01"
    
    sat2 = gm.Satellite("E11")
    assert sat2.prn == 11
    assert sat2.system.name == "Galileo"


def test_epoch():
    """Test Epoch class."""
    import geoveil_mp as gm
    
    epoch = gm.Epoch(2024, 6, 15, 12, 30, 45.5)
    assert epoch.year == 2024
    assert epoch.month == 6
    assert epoch.day == 15
    assert epoch.hour == 12
    assert epoch.minute == 30
    assert abs(epoch.second - 45.5) < 0.001
    
    # Test GPS time conversion
    week, tow = epoch.to_gps_time()
    assert week > 0
    assert 0 <= tow < 604800
    
    # Test Julian date
    jd = epoch.julian_date()
    assert jd > 2400000
    
    # Test day of year
    doy = epoch.day_of_year()
    assert 1 <= doy <= 366


def test_ecef():
    """Test ECEF coordinates."""
    import geoveil_mp as gm
    
    ecef = gm.Ecef(4000000.0, 1000000.0, 4800000.0)
    assert ecef.x == 4000000.0
    assert ecef.y == 1000000.0
    assert ecef.z == 4800000.0
    
    # Test magnitude
    mag = ecef.magnitude()
    assert mag > 6000000  # Should be roughly Earth radius
    
    # Test conversion to geodetic
    geo = ecef.to_geodetic()
    assert -90 <= geo.lat <= 90
    assert -180 <= geo.lon <= 180


def test_geodetic():
    """Test Geodetic coordinates."""
    import geoveil_mp as gm
    
    geo = gm.Geodetic(45.0, 10.0, 100.0)
    assert geo.lat == 45.0
    assert geo.lon == 10.0
    assert geo.height == 100.0
    
    # Test conversion to ECEF and back
    ecef = geo.to_ecef()
    geo2 = ecef.to_geodetic()
    assert abs(geo.lat - geo2.lat) < 0.0001
    assert abs(geo.lon - geo2.lon) < 0.0001
    assert abs(geo.height - geo2.height) < 1.0


def test_frequency():
    """Test frequency calculation."""
    import geoveil_mp as gm
    
    # GPS L1
    freq_l1 = gm.get_frequency("G", 1)
    assert freq_l1 is not None
    assert abs(freq_l1 - 1575.42e6) < 1e3
    
    # GPS L2
    freq_l2 = gm.get_frequency("G", 2)
    assert freq_l2 is not None
    assert abs(freq_l2 - 1227.60e6) < 1e3
    
    # GPS L5
    freq_l5 = gm.get_frequency("G", 5)
    assert freq_l5 is not None
    assert abs(freq_l5 - 1176.45e6) < 1e3


def test_wavelength():
    """Test wavelength calculation."""
    import geoveil_mp as gm
    
    # GPS L1 wavelength should be ~19cm
    wl = gm.get_wavelength("G", 1)
    assert wl is not None
    assert 0.18 < wl < 0.20


def test_calculate_azel():
    """Test azimuth/elevation calculation."""
    import geoveil_mp as gm
    
    # Receiver position (roughly Amsterdam)
    receiver = gm.Ecef(3924000.0, 301000.0, 5002000.0)
    
    # Satellite position (somewhere above)
    satellite = gm.Ecef(15000000.0, 10000000.0, 20000000.0)
    
    az, el = gm.calculate_azel(receiver, satellite)
    
    # Check ranges
    assert 0 <= az < 360
    assert -90 <= el <= 90


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
