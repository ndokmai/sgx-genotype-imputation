# SMac configuration
## 0 for SMac; 1 for SMac-lite
LITE=0
## 0 for simulation-mode; 1 for SGX
SGX=1
## 0 to disable remote attestation; 1 for remote attestation
RA=1

# Enclave configuration
## location of private key to sign enclave
ENCLAVE_HEAP_SIZE=0x20000000
ENCLAVE_STACK_SIZE=0x20000
SP_SIGNING_KEY=service-provider/keys/sp_private_singing_key.pem
