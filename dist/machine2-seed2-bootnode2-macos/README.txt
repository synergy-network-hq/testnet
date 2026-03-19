machine2 macOS bootstrap package
================================

Contents
- seed2/
- bootnode2/
- install_machine2_bootstrap.sh
- restart-machine2-bootstrap-macos.sh

One command after extracting
- cd machine2-seed2-bootnode2-macos
- bash ./install_machine2_bootstrap.sh

Default install targets
- ~/seed2
- ~/bootnode2

Override targets if needed
- SEED_DIR=/custom/seed2 BOOTNODE_DIR=/custom/bootnode2 bash ./install_machine2_bootstrap.sh
