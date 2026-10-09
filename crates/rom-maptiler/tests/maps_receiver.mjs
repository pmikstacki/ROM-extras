// Authored MapTiler-shaped metadata. No account, native service, tile data or renderer claim.
let requests=0,rateRequests=0;
export function mapRequest(req,res,url){
 const parts=url.pathname.split('/');
 if(parts[1]!=='operator'||!['maps','tiles'].includes(parts[2]))return false;
 if(req.method!=='GET'||url.searchParams.get('key')!=='synthetic-secret'||[...url.searchParams].length!==1){res.writeHead(400);res.end();return true;}
 const id=parts[3];
 res.setHeader('content-type','application/json');
 if(id==='rate'){res.writeHead(++rateRequests===1?429:503,{'retry-after':'2'});res.end('synthetic-secret');return true;}
 if(id==='redirect'){res.writeHead(302,{location:'/operator/tiles/vector-0001/tiles.json?key=synthetic-secret'});res.end();return true;}
 if(id==='huge'){res.writeHead(200,{'content-length':'1048577'});res.end();return true;}
 const raster=parts[2]==='maps'&&parts[4]==='256'&&parts[5]==='tiles.json'&&parts.length===6;
 const style=parts[2]==='maps'&&parts[4]==='style.json'&&parts.length===5;
 const vector=parts[2]==='tiles'&&parts[4]==='tiles.json'&&parts.length===5;
 if(!raster&&!style&&!vector){res.writeHead(400);res.end();return true;}
 if((raster||style)&&id!=='map-0001'||vector&&!['vector-0001','slow','bad-credit','bad-geometry','empty'].includes(id)){res.writeHead(400);res.end();return true;}
 const key='?key=synthetic-secret',origin='https://maps.example.test';
 const metadata={tilejson:'2.0.0',attribution:'fixture credit',tiles:[`${origin}/tiles/{z}/{x}/{y}.${raster?'png':'pbf'}${key}`],vector_layers:[{id:'Roads/0001',fields:{request:String(++requests)}}]};
 if(id==='bad-credit')metadata.attribution='<script>synthetic-secret</script>';
 if(id==='bad-geometry')metadata.bounds=[181,0,182,1];
 if(id==='empty')metadata.tiles=[];
 if(style){res.end(JSON.stringify({version:8,sources:{'Base/0001':{type:'vector',url:origin+'/tiles.json'+key}},glyphs:origin+'/fonts/{fontstack}/{range}.pbf'+key,layers:[{id:'道路',type:'line',source:'Base/0001','source-layer':'Roads/0001'}]}));return true;}
 const body=JSON.stringify(metadata);
 if(id==='slow'){setTimeout(()=>res.end(body),250);return true;}
 res.end(body);return true;
}
