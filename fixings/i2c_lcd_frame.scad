$fn=50;

bracket_bezel_infill_thickness = 1;
bracket_bezel_thickness = 5;
difference() {
    union() {
       translate([0,0,bracket_bezel_thickness/2]) cube([89.75, 34.5,bracket_bezel_thickness], center=true);
            
        
        
       translate([0,30/2 + 34.5/2 - 0.01,2 ]) {
           difference() {
           translate([0,0, (bracket_bezel_thickness+1.5)/2]) cube([100,30,bracket_bezel_thickness+1.5],center=true);
           
               //slot hole inner
               hull() {
                   translate([40,-5,-5]) cylinder(d=4.5, h=50);
                   translate([40,5,-5]) cylinder(d=4.5, h=50);
               }
                hull() {
                   translate([40,-5,-1]) cylinder(d=8, h=4);
                   translate([40,5,-1]) cylinder(d=8, h=4);
                }

               
               //slot hole inner
                hull() {
                   translate([-40,-5,-5]) cylinder(d=4.5, h=50);
                   translate([-40,5,-5]) cylinder(d=4.5, h=50);
                }
                 hull() {
                   translate([-40,-5,0]) cylinder(d=8, h=4);
                   translate([-40,5,0]) cylinder(d=8, h=4);
                }
                
           }
       }
    }
  //  translate([(100-65)/2,40,0]) square ([65,70]);

translate([0,0,20/2 +bracket_bezel_infill_thickness]) cube([71, 25, 20], center=true);
translate([0,0,-1]) cube([71-6, 25-9, 50], center=true);


}
