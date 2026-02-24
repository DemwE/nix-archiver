# Testing the NixOS Module

This guide explains how to test the nix-archiver NixOS module.

## Quick Syntax Validation

Check that all Nix files parse correctly:

```bash
# Test module files
nix-instantiate --parse modules/nix-archiver.nix
nix-instantiate --parse modules/options.nix

# Test examples
for f in examples/nixos/*.nix; do
  echo "Testing $f..."
  nix-instantiate --parse "$f"
done
```

## Testing in a NixOS VM

### Option 1: Using nixos-rebuild build-vm

Create a test configuration:

```nix
# test-vm.nix
{ config, pkgs, ... }:

{
  imports = [
    ./modules/nix-archiver.nix
  ];

  # Minimal VM configuration
  boot.loader.grub.device = "/dev/sda";
  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };

  # Enable nix-archiver
  services.nix-archiver = {
    enable = true;
    
    indexer = {
      enable = true;
      updateInterval = "daily";
      maxCommits = 100;  # Small for testing
    };
    
    pinnedPackages = {
      hello = "2.12.1";
    };
  };

  # Enable SSH for easy access
  services.openssh.enable = true;
  users.users.root.password = "test";
}
```

Build and run the VM:

```bash
nixos-rebuild build-vm -I nixos-config=test-vm.nix
./result/bin/run-*-vm
```

### Option 2: Using NixOS Test Framework

Create a test file:

```nix
# test.nix
import <nixpkgs/nixos/tests/make-test-python.nix> ({ pkgs, ... }: {
  name = "nix-archiver-module-test";
  
  nodes.machine = { config, pkgs, ... }: {
    imports = [ ./modules/nix-archiver.nix ];
    
    services.nix-archiver = {
      enable = true;
      
      indexer = {
        enable = true;
        maxCommits = 50;  # Quick test
      };
      
      pinnedPackages = {
        hello = "2.12.1";
      };
    };
  };
  
  testScript = ''
    machine.wait_for_unit("multi-user.target")
    
    # Check service exists
    machine.succeed("systemctl status nix-archiver-indexer.service --no-pager")
    machine.succeed("systemctl status nix-archiver-indexer.timer --no-pager")
    
    # Check user was created
    machine.succeed("id nix-archiver")
    
    # Check directories exist
    machine.succeed("test -d /var/lib/nix-archiver")
    
    # Trigger indexer manually
    machine.succeed("systemctl start nix-archiver-indexer.service")
    machine.wait_for_unit("nix-archiver-indexer.service")
    
    # Check database was created
    machine.succeed("test -d /var/lib/nix-archiver/db")
    
    # Test CLI
    machine.succeed("nix-archiver stats")
    machine.succeed("nix-archiver search hello")
    
    # Check overlay was generated
    machine.succeed("test -f /var/lib/nix-archiver/overlay.nix")
  '';
})
```

Run the test:

```bash
nix-build test.nix
```

## Manual Testing Checklist

After building the VM and logging in:

### 1. Service Status
```bash
# Check service status
systemctl status nix-archiver-indexer.service
systemctl status nix-archiver-indexer.timer

# View logs
journalctl -u nix-archiver-indexer.service -n 50
```

### 2. User and Permissions
```bash
# Check user exists
id nix-archiver

# Check directory ownership
ls -la /var/lib/nix-archiver

# Verify permissions
stat /var/lib/nix-archiver
```

### 3. Repository and Database
```bash
# Check repository was cloned
ls -la /var/lib/nix-archiver/nixpkgs

# Check database exists
ls -la /var/lib/nix-archiver/db

# View database stats
nix-archiver stats
```

### 4. Indexer Functionality
```bash
# Manually trigger indexer
sudo systemctl start nix-archiver-indexer.service

# Wait for completion
systemctl status nix-archiver-indexer.service

# Check logs for errors
journalctl -u nix-archiver-indexer.service -f
```

### 5. CLI Commands
```bash
# Search for packages
nix-archiver search nodejs
nix-archiver search --version "20.*" nodejs

# Show statistics
nix-archiver stats

# Generate overlay manually
nix-archiver generate -o /tmp/test-overlay.nix
cat /tmp/test-overlay.nix
```

### 6. Overlay Generation
```bash
# Check if overlay was generated
ls -la /var/lib/nix-archiver/overlay.nix

# View overlay content
cat /var/lib/nix-archiver/overlay.nix

# Check if packages.nix was created
cat /var/lib/nix-archiver/packages.nix
```

### 7. Pinned Packages
```bash
# Check if pinned package is available
nix-instantiate --eval -E 'with import <nixpkgs> {}; hello.version'

# Try building a pinned package
nix-build '<nixpkgs>' -A hello
```

### 8. Timer Functionality
```bash
# Check timer schedule
systemctl status nix-archiver-indexer.timer

# List upcoming timer activations
systemctl list-timers nix-archiver-indexer.timer

# Force timer to run
sudo systemctl start nix-archiver-indexer.timer
```

## Expected Results

✅ **Service installed**: `nix-archiver-indexer.service` exists  
✅ **Timer active**: `nix-archiver-indexer.timer` is running  
✅ **User created**: `nix-archiver` user and group exist  
✅ **Directories created**: `/var/lib/nix-archiver/{db,nixpkgs}` exist  
✅ **Repository cloned**: Nixpkgs repository in state directory  
✅ **Database populated**: Packages indexed in Sled database  
✅ **Overlay generated**: `/var/lib/nix-archiver/overlay.nix` created  
✅ **CLI works**: `nix-archiver` command accessible  
✅ **Logs clean**: No errors in systemd journal  

## Common Issues

### Issue: Service fails to start
**Check**: 
```bash
journalctl -u nix-archiver-indexer.service -n 100
systemctl status nix-archiver-indexer.service -l
```
**Common causes**:
- Network issues (can't clone Nixpkgs)
- Permission problems (state directory not writable)
- Database locked (another instance running)

### Issue: Timer doesn't run
**Check**:
```bash
systemctl status nix-archiver-indexer.timer
systemctl list-timers
```
**Fix**: Timer might need to be enabled manually in test VM

### Issue: Overlay not generated
**Check**:
```bash
ls -la /var/lib/nix-archiver/
cat /var/lib/nix-archiver/packages.nix
```
**Possible causes**:
- No pinned packages configured
- `generateOverlay = false` in config
- Service hasn't run yet

### Issue: CLI can't find database
**Check**:
```bash
echo $NIX_ARCHIVER_DB
ls -la /var/lib/nix-archiver/db
```
**Fix**: Module sets environment variable automatically when service is enabled

## Automated Testing

TODO: Implement comprehensive NixOS test suite covering:
- [ ] Service activation and startup
- [ ] Timer scheduling and execution  
- [ ] Repository cloning and updates
- [ ] Database creation and indexing
- [ ] Overlay generation and application
- [ ] CLI functionality
- [ ] Permission and security checks
- [ ] Error handling and recovery
- [ ] Configuration changes and reloads

## Next Steps

After manual testing is successful:
1. Create automated NixOS tests
2. Add CI/CD pipeline for testing
3. Test on different NixOS versions
4. Prepare nixpkgs pull request
5. Gather community feedback

## See Also

- [NixOS Test Documentation](https://nixos.org/manual/nixos/stable/index.html#sec-nixos-tests)
- [Module Documentation](modules/README.md)
- [Example Configurations](examples/nixos/)
