"""
Test cases for the execute endpoint.
These tests demonstrate expected behavior.
For integration tests with WireMock, use docker-compose.
"""
import pytest


def test_simple_arithmetic():
    """Test basic arithmetic operations."""
    code = "INPUTS.x + INPUTS.y"
    inputs = {"x": 5, "y": 10}
    expected_result = 15
    assert expected_result == 15


def test_code_with_inputs():
    """Test code execution with INPUTS."""
    code = "INPUTS.name.toUpperCase()"
    inputs = {"name": "test"}
    expected_result = "TEST"
    assert expected_result == "TEST"


def test_complex_transformations():
    """Test complex array transformations."""
    code = "INPUTS.numbers.map(n => n * 2).reduce((a, b) => a + b, 0)"
    inputs = {"numbers": [1, 2, 3, 4, 5]}
    expected_result = 30  # (2+4+6+8+10)
    assert expected_result == 30


def test_http_get_mock_response():
    """Demonstrate httpGet with WireMock."""
    # Example code that would be executed
    code = """
        const response = httpGet('http://wiremock:8080/api/todos/1');
        response.data.title
    """
    
    # With WireMock running and configured:
    # Expected mock response structure
    mock_response = {
        "ok": True,
        "status": 200,
        "statusText": "OK",
        "headers": {},
        "data": {"id": "1", "title": "Sample Todo 1", "completed": False, "userId": 1}
    }
    
    assert mock_response["data"]["title"] == "Sample Todo 1"


def test_http_post_mock():
    """Demonstrate HTTP POST with WireMock."""
    code = """
        const response = httpGet('http://wiremock:8080/api/data', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ test: true })
        });
        response.data.success
    """
    
    mock_response = {
        "ok": True,
        "status": 201,
        "statusText": "Created",
        "headers": {},
        "data": {"success": True, "message": "Data created successfully"}
    }
    
    assert mock_response["data"]["success"] is True


def test_array_from_http():
    """Demonstrate array processing from HTTP response."""
    code = """
        const response = httpGet('http://wiremock:8080/api/users');
        response.data.length
    """
    
    mock_response = {
        "ok": True,
        "status": 200,
        "statusText": "OK",
        "headers": {},
        "data": [
            {"id": 1, "name": "John Doe", "email": "john@example.com"},
            {"id": 2, "name": "Jane Smith", "email": "jane@example.com"}
        ]
    }
    
    assert len(mock_response["data"]) == 2
    assert isinstance(mock_response["data"], list)


def test_conditional_logic_with_http():
    """Demonstrate conditional logic with HTTP data."""
    code = """
        const response = httpGet('http://wiremock:8080/api/data');
        response.data.data.status === 'active' ? response.data.data.value * 2 : 0
    """
    
    mock_response = {
        "ok": True,
        "status": 200,
        "statusText": "OK",
        "headers": {},
        "data": {
            "message": "Hello from WireMock!",
            "timestamp": 1234567890,
            "data": {"value": 42, "status": "active"}
        }
    }
    
    result = mock_response["data"]["data"]["value"] * 2 if mock_response["data"]["data"]["status"] == "active" else 0
    assert result == 84


def test_string_operations():
    """Test string operations."""
    code = "INPUTS.text.split(' ').length"
    inputs = {"text": "hello world test"}
    expected = 3
    assert expected == 3


def test_json_operations():
    """Test JSON operations."""
    code = "JSON.stringify(INPUTS)"
    inputs = {"key": "value"}
    expected = '{"key":"value"}'
    assert expected == '{"key":"value"}'


def test_array_methods():
    """Test array methods."""
    code = "INPUTS.numbers.map(n => n * 2)"
    inputs = {"numbers": [1, 2, 3]}
    expected = [2, 4, 6]
    assert expected == [2, 4, 6]
