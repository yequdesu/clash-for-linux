[Unit]
Description=Clashctl Proxy Service (Mihomo)
After=network.target NetworkManager.service systemd-networkd.service iwd.service
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=simple
User=%u
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE
Restart=always
RestartSec=3
ExecStart=__KERNEL_PATH__ -d __RESOURCES_DIR__ -f __RESOURCES_DIR__/runtime.yaml
ExecStop=/bin/kill -SIGTERM $MAINPID
StandardOutput=append:__LOG_FILE__
StandardError=append:__LOG_FILE__

[Install]
WantedBy=multi-user.target
