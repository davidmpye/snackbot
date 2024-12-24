$fn=100;
difference() {
    union() { 
        //outer part of bezel 
        translate([0,0,1]) cube([106,85.5,2],center=true);
        //outer frame with mounts
        difference() {
            //outer frame
               translate([0,0,2.5 + 1]) cube([107.5+30, 87+15,5], center=true);
            //Slots lower
            hull() {
                translate([-56,25.5,0]) cylinder(d=4.5,h=10);
                translate([-70,25.5,0]) cylinder(d=4.5,h=10);
            }
            hull() {
                translate([-56,-25.5,0]) cylinder(d=4.5,h=10);
                translate([-70,-25.5,0]) cylinder(d=4.5,h=10);
            }
            //slots upper
            hull() {
                translate([58,40,0]) cylinder(d=4.5,h=10);
                translate([58,50,0]) cylinder(d=4.5,h=10);

            }
            hull() {
                translate([58,-40,0]) cylinder(d=4.5,h=10);
                translate([58,-50,0]) cylinder(d=4.5,h=10);
            }
        }
    }
    //viewing aperture
    translate([2.5,0,5]) cube([88.1,53.5,10], center=true);
    //big hole
    translate([0,0,10 + 1])     cube([98.5, 59, 20],center=true);
    
    
    //screw holes to retain
    translate([98.5/2 + 2,59/2 +2, 2]) cylinder(d=1.5, h= 50);
    translate([-98.5/2 - 2,59/2 +2, 2]) cylinder(d=1.5, h= 50);


    translate([-98.5/2 - 2, -59/2 -2, 2]) cylinder(d=1.5, h= 50);
    translate([98.5/2 + 2, -59/2 -2, 2]) cylinder(d=1.5, h= 50);

    
};




//LCD WINDOW

//square ([90.5, 115.5]);


