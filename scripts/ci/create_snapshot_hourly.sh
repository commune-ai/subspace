#!/bin/bash

TAR_SRC=/mnt/chainstorage/node-/chains/nakamoto_mainnet/db/full
SNAPSHOT_TMP="tmp"

git -C /root/ pull origin master && cargo build --release --manifest-path /root/subspace/Cargo.toml

currenthour=$(date +%H)
if [[ "$currenthour" == "00" ]]; then
	echo "[+] It's midnight, creating nightly snapshot"

	SNAPSHOT_FILENAME=snapshot_$(date +"%m-%d-%Y_%H")-00-nightly
	TAR_TARGET=/mnt/snapshots_nightly/$SNAPSHOT_FILENAME.tar.gz
	SNAPSHOT_DIR="snapshots_nightly"

	# Let's delete the oldest snapshot in here first.
	# we only need to maintain 2 days at a time.
	cd /mnt/$SNAPSHOT_DIR
	ls -1t | tail -n +3 | xargs rm
	cd ~
else
	echo "[+] Creating hourly snapshot"
	SNAPSHOT_FILENAME=snapshot_$(date +"%m-%d-%Y_%H")-00
	TAR_TARGET=/mnt/snapshots_hourly/$SNAPSHOT_FILENAME.tar.gz
	SNAPSHOT_DIR="snapshots_hourly"

	# Let's delete the oldest snapshot in here first.
	# we only need to maintain 10 hours at a time.
	cd /mnt/$SNAPSHOT_DIR
	ls -1t | tail -n +2 | xargs rm
	cd ~
fi

echo "[+] Removing previous docker images from previous build"
# Kill dangling docker images from previous builds
/usr/bin/docker system prune -a -f

echo "[+] Stopping Subspace and starting database export"
# Stop  and start the DB export
#/usr/local/bin/pm2 describe  > /dev/null
RUNNING=`/usr/local/bin/pm2 pid `

# If  is running, then start the export process.
# NOTE: Export won't happen if chain is down, because it would very likely be out of date.
if [ "${RUNNING}" -ne 0 ]; then
	
	echo "[+] Stopping  PM2 job"
	# Stop  chain so we can export the db
	/usr/local/bin/pm2 stop 
	cd $TAR_SRC
	tar -zcvf $TAR_TARGET *
	
	# COPY TO TMP FOLDER
	cp $TAR_TARGET /root/$SNAPSHOT_TMP

	# Check if we tarred properly.
	if [[ $(tar ztvf $TAR_TARGET | grep "MANIFEST-*") ]]; then 

		# Add to ipfs
		#NEW_PIN=`ipfs add -Q -r /root/snapshots_hourly`
		#ipfs name publish /ipfs/$NEW_PIN
		cd ~		
		# Build docker image
		echo "[+] Building Docker image from directory ${SNAPSHOT_TMP} and snapshot file ${SNAPSHOT_FILENAME}"
		DOCKER_BUILDKIT=1 /usr/bin/docker build -t  . --platform linux/x86_64 --build-arg SNAPSHOT_DIR=$SNAPSHOT_TMP --build-arg SNAPSHOT_FILE=$SNAPSHOT_FILENAME  -f /root/subspace/Dockerfile --squash

		# Tag new image with latest
		echo "[+] Tagging new image with latest tag"
		/usr/bin/docker tag  opentensorfdn/subspace:latest
		/usr/bin/docker tag  opentensorfdn/subspace:$SNAPSHOT_FILENAME
	
		# now let's push this sum' bitch to dockerhub
		echo "[+] Pushing Docker image to DockerHub"
		/usr/bin/docker push opentensorfdn/:latest
		/usr/bin/docker push opentensorfdn/:$SNAPSHOT_FILENAME
	
		# Start the chain again
		echo "[+] Restarting Subspace chain"
		/usr/local/bin/pm2 start  --watch

		# Clear tmp file
		echo "[+] clearing tmp file"
		rm -rf /root/tmp/*
	else
		echo "[-] Tar file is corrupt"
	fi
	cd ~  
	# Empty the snapshot tmp folder
	rm -rf $SNAPSHOT_TMP/*
else
	echo "[+] Subspace is already stopped!"
fi
echo "\n"
