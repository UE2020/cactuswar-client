all:
		wasm-pack build --target=web
		shost 9000

run: all

release: 
		wasm-pack build --target=web
		rm -rf deployment
		mkdir deployment
		cp index.html deployment/index.html
		cp banner_large.png deployment/banner_large.png
		cp 'dHJ1c3Q=.js' deployment/
		cp -r pkg/ deployment
		rm -rf deployment/pkg/*.ts deployment/pkg/package.json deployment/pkg/.gitignore
		@echo "Deployment in deployment/"
		
