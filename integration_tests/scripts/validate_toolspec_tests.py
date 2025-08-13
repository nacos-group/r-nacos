#!/usr/bin/env python3
"""
Validation script for ToolSpec integration tests

This script validates that the integration test files are properly structured
and can be imported without errors.
"""

import sys
import importlib.util
from pathlib import Path


def validate_script(script_path: Path) -> bool:
    """Validate a Python script can be imported"""
    try:
        spec = importlib.util.spec_from_file_location("test_module", script_path)
        if spec is None:
            print(f"❌ Could not create spec for {script_path}")
            return False
        
        module = importlib.util.module_from_spec(spec)
        if module is None:
            print(f"❌ Could not create module for {script_path}")
            return False
        
        # Try to execute the module (this will catch syntax errors)
        spec.loader.exec_module(module)
        print(f"✅ {script_path.name} - syntax and imports OK")
        return True
        
    except Exception as e:
        print(f"❌ {script_path.name} - validation failed: {e}")
        return False


def main():
    """Main validation function"""
    script_dir = Path(__file__).parent
    
    # Scripts to validate
    scripts = [
        script_dir / "toolspec_console_api_test.py",
        script_dir / "run_toolspec_test.py"
    ]
    
    print("🔍 Validating ToolSpec integration test scripts...")
    print("=" * 60)
    
    all_valid = True
    
    for script in scripts:
        if not script.exists():
            print(f"❌ {script.name} - file not found")
            all_valid = False
            continue
        
        if not validate_script(script):
            all_valid = False
    
    print("=" * 60)
    
    if all_valid:
        print("🎉 All validation checks passed!")
        print("\nNext steps:")
        print("1. Build r-nacos: cd r-nacos && cargo build")
        print("2. Run tests: ./run_toolspec_test.py")
        return 0
    else:
        print("💥 Some validation checks failed!")
        return 1


if __name__ == "__main__":
    sys.exit(main())